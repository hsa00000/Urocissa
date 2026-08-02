use crate::public::constant::storage::get_data_path;
use crate::public::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_upload::GuardUpload;
use crate::router::{AppResult, GuardResult};
use crate::tasks::actor::deduplicate::PresignedMetadata;
use crate::workflow::index_with_presigned_metadata;
use anyhow::Result;
use arrayvec::ArrayString;
use rocket::form::{Errors, Form};
use rocket::fs::TempFile;
use std::path::{Path, PathBuf};
use tokio::task::spawn_blocking;
use uuid::Uuid;

/// Data structure representing the multipart form for file uploads.
#[derive(FromForm, Debug)]
pub struct UploadForm<'r> {
    /// Sequential list of uploaded files.
    #[field(name = "file")]
    pub files: Vec<TempFile<'r>>,

    /// Timestamps (Unix epoch in milliseconds) corresponding to each file by index.
    #[field(name = "lastModified")]
    pub last_modified: Vec<u64>,
}

fn get_filename(file: &TempFile<'_>) -> String {
    file.name()
        .map(std::string::ToString::to_string)
        .unwrap_or_default()
}

fn parse_presigned_list(raw: Option<String>, field: &str) -> AppResult<Vec<String>> {
    raw.map_or_else(
        || Ok(Vec::new()),
        |value| {
            serde_json::from_str::<Vec<String>>(&value).map_err(|error| {
                AppError::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid {field} JSON array: {error}"),
                )
            })
        },
    )
}

fn build_presigned_metadata(
    legacy_album_id: Option<String>,
    album_ids_json: Option<String>,
    tags_json: Option<String>,
) -> AppResult<PresignedMetadata> {
    let mut metadata = PresignedMetadata::default();
    let mut album_ids = parse_presigned_list(album_ids_json, "presigned albums")?;
    if let Some(album_id) = legacy_album_id {
        album_ids.push(album_id);
    }

    for album_id in album_ids {
        let parsed = ArrayString::from(&album_id)
            .map_err(|_| AppError::new(ErrorKind::InvalidInput, "Album ID exceeds 64 bytes"))?;
        metadata.album_ids.insert(parsed);
    }

    for tag in parse_presigned_list(tags_json, "presigned tags")? {
        let tag = tag.trim();
        if !tag.is_empty() {
            metadata.tags.insert(tag.to_owned());
        }
    }
    Ok(metadata)
}

#[post(
    "/upload?<presigned_album_id_opt>&<presigned_album_ids_opt>&<presigned_tags_opt>",
    data = "<form>"
)]
#[allow(clippy::similar_names)]
pub async fn upload(
    auth: GuardResult<GuardUpload>,
    read_only_mode: GuardResult<GuardReadOnlyMode>,
    presigned_album_id_opt: Option<String>,
    presigned_album_ids_opt: Option<String>,
    presigned_tags_opt: Option<String>,
    form: Result<Form<UploadForm<'_>>, Errors<'_>>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;

    let mut inner_form = match form {
        Ok(f) => f.into_inner(),
        Err(errors) => {
            // Flatten generic Rocket errors into a single context for debugging
            let error_msg = errors
                .iter()
                .fold(String::from("Form parsing failed: "), |acc, e| {
                    format!("{acc}; {e}")
                });
            return Err(AppError::new(ErrorKind::InvalidInput, error_msg));
        }
    };

    let presigned = build_presigned_metadata(
        presigned_album_id_opt,
        presigned_album_ids_opt,
        presigned_tags_opt,
    )?;

    // Ensure strict 1:1 mapping between files and metadata
    if inner_form.files.len() != inner_form.last_modified.len() {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "Mismatch between file count and timestamp count.",
        ));
    }

    for (i, file) in inner_form.files.iter_mut().enumerate() {
        let last_modified = inner_form.last_modified[i];
        let filename = get_filename(file);
        let extension = get_extension(file)?;

        if VALID_IMAGE_EXTENSIONS.contains(&extension.as_str())
            || VALID_VIDEO_EXTENSIONS.contains(&extension.as_str())
        {
            let final_path = save_file(file, filename, extension, last_modified).await?;
            index_with_presigned_metadata(PathBuf::from(final_path), presigned.clone())
                .await
                .or_raise(|| (ErrorKind::Internal, "Failed to index file"))?;
        } else {
            error!("Rejected invalid file type: {}", extension);
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("Invalid file type: {extension}"),
            ));
        }
    }

    Ok(())
}

/// Persists the temporary file to disk with the correct modification time.
///
/// Returns the absolute path of the saved file.
async fn save_file(
    file: &mut TempFile<'_>,
    filename: String,
    extension: String,
    last_modified_ms: u64,
) -> Result<String, AppError> {
    let unique_id = Uuid::new_v4();
    let root = get_data_path();
    let upload_dir = root.join("upload");

    // Ensure upload directory exists (though it should be created at init)
    if !upload_dir.exists() {
        std::fs::create_dir_all(&upload_dir).map_err(|e| {
            AppError::new(ErrorKind::IO, format!("Failed to create upload dir: {e}"))
        })?;
    }

    let tmp_path = upload_dir.join(format!("{filename}-{unique_id}.tmp"));

    // Move to a temp location first to avoid blocking the async runtime with IO
    file.move_copy_to(&tmp_path)
        .await
        .or_raise(|| (ErrorKind::IO, "Failed to move temporary file"))?;

    let filename_owned = filename.clone();
    let tmp_path_owned = tmp_path.clone();

    // Perform metadata operations and rename in a blocking thread.
    // 1. Set mtime on the .tmp file.
    // 2. Atomic rename to .ext (final state).
    // This ensures the file watcher (workflow) only picks up the file once it is fully written and has the correct timestamp.
    let final_path = spawn_blocking(move || -> Result<String, AppError> {
        let final_path = upload_dir.join(format!("{filename_owned}-{unique_id}.{extension}"));

        set_last_modified_time(&tmp_path_owned, last_modified_ms)?;
        std::fs::rename(&tmp_path_owned, &final_path)
            .or_raise(|| (ErrorKind::IO, "Failed to rename file"))?;

        Ok(final_path.to_string_lossy().into_owned())
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;

    Ok(final_path)
}

#[allow(clippy::cast_possible_wrap)]
fn set_last_modified_time(path: &Path, last_modified_ms: u64) -> Result<(), AppError> {
    let mtime = filetime::FileTime::from_unix_time((last_modified_ms / 1000) as i64, 0);
    filetime::set_file_mtime(path, mtime)
        .or_raise(|| (ErrorKind::IO, "Failed to set file modification time"))?;
    Ok(())
}

fn get_extension(file: &TempFile<'_>) -> Result<String, AppError> {
    file.content_type()
        .and_then(|ct| ct.extension())
        .map(|ext| ext.as_str().to_lowercase())
        .ok_or_else(|| {
            error!("Failed to determine file extension from Content-Type");
            AppError::new(ErrorKind::InvalidInput, "Missing or unknown file extension")
        })
}

#[cfg(test)]
mod presigned_metadata_tests {
    use super::build_presigned_metadata;

    #[test]
    fn merges_legacy_and_json_presets() {
        let metadata = build_presigned_metadata(
            Some("legacy-album".to_owned()),
            Some(r#"["album-a","album-b","album-a"]"#.to_owned()),
            Some(r#"[" first ","second","first",""]"#.to_owned()),
        )
        .unwrap();

        assert_eq!(metadata.album_ids.len(), 3);
        assert!(
            metadata
                .album_ids
                .iter()
                .any(|id| id.as_str() == "legacy-album")
        );
        assert!(metadata.album_ids.iter().any(|id| id.as_str() == "album-a"));
        assert!(metadata.album_ids.iter().any(|id| id.as_str() == "album-b"));
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.tags.contains("first"));
        assert!(metadata.tags.contains("second"));
    }

    #[test]
    fn rejects_malformed_presigned_json() {
        assert!(build_presigned_metadata(None, Some("not-json".to_owned()), None).is_err());
    }
}
