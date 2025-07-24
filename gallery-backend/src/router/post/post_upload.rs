use crate::public::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};
use crate::router::AppResult;
use crate::router::claims::claims::Role;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_upload::GuardUpload;
use crate::workflow::{index_for_upload, index_for_watch};
use anyhow::Result;
use anyhow::bail;
use rocket::form::{self, DataField, FromFormField, ValueField};
use rocket::{form::Form, fs::TempFile};
use std::path::PathBuf;
use std::time::Instant;
use tokio::task::spawn_blocking;
use uuid::Uuid;
pub enum FileUpload<'r> {
    LastModified(u64),
    File(TempFile<'r>),
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for FileUpload<'r> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        // Use the from_value method already implemented for u64
        match u64::from_value(field) {
            Ok(value) => Ok(FileUpload::LastModified(value)),
            Err(err) => Err(err),
        }
    }

    async fn from_data(field: DataField<'r, '_>) -> form::Result<'r, Self> {
        // Use the from_data method already implemented for TempFile
        match TempFile::from_data(field).await {
            Ok(temp_file) => Ok(FileUpload::File(temp_file)),
            Err(err) => Err(err),
        }
    }
}

fn get_filename(file: &TempFile<'_>) -> String {
    file.name()
        .map(|name| name.to_string())
        .unwrap_or_else(|| "".to_string())
}

#[post("/upload", data = "<data>")]
pub async fn upload(
    auth: GuardUpload,
    _read_only_mode: GuardReadOnlyMode,
    data: Form<Vec<FileUpload<'_>>>,
) -> AppResult<()> {
    let mut last_modified_time = 0;

    for file_data in data.into_inner() {
        match file_data {
            FileUpload::LastModified(last_modified_time_received) => {
                last_modified_time = last_modified_time_received;
            }
            FileUpload::File(mut file) => {
                let start_time = Instant::now();
                let filename = get_filename(&file);
                let extension = get_extension(&file)?;

                warn!(duration = &*format!("{:?}", start_time.elapsed()); "Get filename and extension");
                if let Role::Share(ref resolved_share) = auth.claims.role
                    && (VALID_IMAGE_EXTENSIONS.contains(&extension.as_str())
                        || VALID_VIDEO_EXTENSIONS.contains(&extension.as_str()))
                {
                    let final_path =
                        save_file(&mut file, filename, extension, last_modified_time).await?;
                    index_for_upload(PathBuf::from(final_path), resolved_share.album_id).await?;
                } else if VALID_IMAGE_EXTENSIONS.contains(&extension.as_str())
                    || VALID_VIDEO_EXTENSIONS.contains(&extension.as_str())
                {
                    let final_path =
                        save_file(&mut file, filename, extension, last_modified_time).await?;
                    index_for_watch(PathBuf::from(final_path)).await?;
                } else {
                    error!("Invalid file type");
                    return Err(anyhow::anyhow!("Invalid file type: {}", extension).into());
                }
            }
        }
    }
    Ok(())
}
async fn save_file(
    file: &mut TempFile<'_>,
    filename: String,
    extension: String,
    last_modified_time: u64,
) -> Result<String> {
    let unique_id = Uuid::new_v4();
    let path_tmp = format!("./upload/{}-{}.tmp", filename, unique_id);

    file.move_copy_to(&path_tmp).await?;

    let filename = filename.clone(); // Needed because filename is moved in path_tmp

    let path_final = spawn_blocking(move || -> Result<String> {
        let path_final = format!("./upload/{}-{}.{}", filename, unique_id, extension);
        set_last_modified_time(&path_tmp, last_modified_time)?;
        std::fs::rename(&path_tmp, &path_final)?;
        Ok(path_final)
    })
    .await??;

    Ok(path_final)
}
fn set_last_modified_time(path: &str, last_modified_time: u64) -> Result<()> {
    let mtime = filetime::FileTime::from_unix_time((last_modified_time / 1000) as i64, 0);
    filetime::set_file_mtime(path, mtime)?;
    Ok(())
}

pub fn get_extension(file: &TempFile<'_>) -> Result<String> {
    match file.content_type() {
        Some(ct) => match ct.extension() {
            Some(ext) => Ok(ext.as_str().to_lowercase()),
            None => {
                error!("Failed to extract file extension.");
                bail!("Failed to extract file extension.")
            }
        },
        None => {
            error!("Failed to get content type.");
            bail!("Failed to get content type.")
        }
    }
}
