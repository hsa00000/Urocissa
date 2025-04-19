use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::Responder;
use rocket_seek_stream::SeekStream;
use std::path::{Path, PathBuf};

use crate::router::fairing::{
    guard_auth::GuardAuthShare,
    guard_hash::{GuardHash, GuardHashOriginal},
};
#[derive(Responder)]
pub enum CompressedFileResponse<'a> {
    SeekStream(SeekStream<'a>),
    NamedFile(NamedFile),
}

#[get("/object/compressed/<file_path..>")]
pub async fn compressed_file(
    _auth_guard: GuardAuthShare,
    _hash_guard: GuardHash,
    file_path: PathBuf,
) -> Result<CompressedFileResponse<'static>, Status> {
    let compressed_file_path = Path::new("./object/compressed").join(&file_path);
    if compressed_file_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        == Some("mp4")
    {
        SeekStream::from_path(compressed_file_path)
            .map(CompressedFileResponse::SeekStream)
            .map_err(|error| {
                error!("Error opening MP4 file: {:#?}", error);
                Status::NotFound
            })
    } else {
        NamedFile::open(compressed_file_path)
            .await
            .map(CompressedFileResponse::NamedFile)
            .map_err(|error| {
                error!("Error opening file: {:#?}", error);
                Status::NotFound
            })
    }
}

#[get("/object/imported/<file_path..>")]
pub async fn imported_file(
    _auth: GuardAuthShare,
    _hash_guard: GuardHashOriginal,
    file_path: PathBuf,
) -> Result<CompressedFileResponse<'static>, Status> {
    let imported_file_path = Path::new("./object/imported").join(&file_path);
    NamedFile::open(imported_file_path)
        .await
        .map(CompressedFileResponse::NamedFile)
        .map_err(|error| {
            error!("Error opening imported file: {:#?}", error);
            Status::NotFound
        })
}
