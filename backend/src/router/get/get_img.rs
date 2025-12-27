use crate::router::{
    AppResult, GuardResult,
    fairing::{
        guard_hash::{GuardHash, GuardHashOriginal},
        guard_share::GuardShare,
    },
};
use anyhow::Context;
use rocket::fs::NamedFile;
use rocket::response::Responder;
use rocket_seek_stream::SeekStream;
use std::path::{Path, PathBuf};
#[derive(Responder)]
pub enum CompressedFileResponse<'a> {
    SeekStream(SeekStream<'a>),
    NamedFile(NamedFile),
}

#[get("/object/compressed/<file_path..>")]
pub async fn compressed_file(
    auth_guard: GuardResult<GuardShare>,
    hash_guard: GuardResult<GuardHash>,
    file_path: PathBuf,
) -> AppResult<CompressedFileResponse<'static>> {
    let _ = auth_guard?;
    let _ = hash_guard?;
    let compressed_file_path = Path::new("./object/compressed").join(&file_path);

    let result = match compressed_file_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
    {
        Some("mp4") => SeekStream::from_path(&compressed_file_path)
            .map(CompressedFileResponse::SeekStream)
            .context(format!(
                "Failed to open MP4 file: {}",
                compressed_file_path.display()
            ))?,
        Some("jpg") => {
            let named_file = NamedFile::open(&compressed_file_path)
                .await
                .context(format!(
                    "Failed to open JPG file: {}",
                    compressed_file_path.display()
                ))?;
            CompressedFileResponse::NamedFile(named_file)
        }
        Some(ext) => {
            return Err(anyhow::anyhow!("Unsupported file extension: {}", ext)
                .context(format!("File path: {}", compressed_file_path.display()))
                .into());
        }
        None => {
            return Err(anyhow::anyhow!("File has no extension")
                .context(format!("File path: {}", compressed_file_path.display()))
                .into());
        }
    };

    Ok(result)
}

#[get("/object/imported/<file_path..>")]
pub async fn imported_file(
    auth: GuardResult<GuardShare>,
    hash_guard: GuardResult<GuardHashOriginal>,
    file_path: PathBuf,
) -> AppResult<CompressedFileResponse<'static>> {
    let _ = auth?;
    let _ = hash_guard?;
    let imported_file_path = Path::new("./object/imported").join(&file_path);
    NamedFile::open(imported_file_path)
        .await
        .map(CompressedFileResponse::NamedFile)
        .map_err(|error| {
            error!("Error opening imported file: {:#?}", error);
            anyhow::anyhow!("Error opening imported file: {:#?}", error).into()
        })
}
