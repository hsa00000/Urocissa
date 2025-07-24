use crate::public::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::workflow::index_for_watch;
use anyhow::{Result, anyhow, bail};
use arrayvec::ArrayString;
use rocket::form::Form;
use rocket::fs::TempFile;
use std::path::PathBuf;
use std::time::Instant;
use tokio::task::spawn_blocking;
use uuid::Uuid;

#[derive(FromForm, Debug)]
pub struct UploadForm<'r> {
    /// 依序收到的多個檔案
    #[field(name = "file")]
    pub files: Vec<TempFile<'r>>,

    /// 與檔案順序對應的 lastModified 時戳
    #[field(name = "lastModified")]
    pub last_modified: Vec<u64>,
}

fn get_filename(file: &TempFile<'_>) -> String {
    file.name()
        .map(|name| name.to_string())
        .unwrap_or_else(|| "".to_string())
}

#[post("/upload?<presigned_album_id_opt>", data = "<form>")]
pub async fn upload(
    _auth: GuardAuth,
    _read_only_mode: GuardReadOnlyMode,
    presigned_album_id_opt: Option<String>,
    form: Form<UploadForm<'_>>,
) -> AppResult<()> {
    let mut inner_form = form.into_inner();

    let presigned_album_id_opt: Option<ArrayString<64>> = if let Some(s) = presigned_album_id_opt {
        Some(
            ArrayString::from(&s)
                .map_err(|_| anyhow!("Failed to create ArrayString from presigned_album_id_opt"))?,
        )
    } else {
        None
    };

    if inner_form.files.len() != inner_form.last_modified.len() {
        return Err(
            anyhow!("Mismatch between number of files and lastModified timestamps.").into(),
        );
    }

    for (i, file) in inner_form.files.iter_mut().enumerate() {
        let last_modified_time = inner_form.last_modified[i];
        let start_time = Instant::now();
        let filename = get_filename(file);
        let extension = get_extension(file)?;

        warn!(duration = &*format!("{:?}", start_time.elapsed()); "Get filename and extension");
        if VALID_IMAGE_EXTENSIONS.contains(&extension.as_str())
            || VALID_VIDEO_EXTENSIONS.contains(&extension.as_str())
        {
            let final_path = save_file(file, filename, extension, last_modified_time).await?;
            index_for_watch(PathBuf::from(final_path), presigned_album_id_opt).await?;
        } else {
            error!("Invalid file type");
            return Err(anyhow::anyhow!("Invalid file type: {}", extension).into());
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
