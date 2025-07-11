use std::panic::Location;
use std::sync::Arc;
use std::time::Instant;

use crate::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};
use crate::public::error_data::{ErrorData, handle_error};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::synchronizer::event::{EVENTS_SENDER, ScanQueue};
use rocket::form::{self, DataField, FromFormField, ValueField};
use rocket::http::Status;
use rocket::{form::Form, fs::TempFile};
use tokio::sync::Notify;
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

fn get_extension(file: &TempFile<'_>) -> Result<String, Status> {
    match file.content_type() {
        Some(ct) => match ct.extension() {
            Some(ext) => Ok(ext.as_str().to_lowercase()),
            None => {
                error!("Failed to extract file extension.");
                Err(Status::InternalServerError)
            }
        },
        None => {
            error!("Failed to get content type.");
            Err(Status::InternalServerError)
        }
    }
}

#[post("/upload", data = "<data>")]
pub async fn upload(
    _auth: GuardAuth,
    _read_only_mode: GuardReadOnlyMode,
    data: Form<Vec<FileUpload<'_>>>,
) -> Result<(), Status> {
    let mut last_modified_time = 0;

    for file_data in data.into_inner() {
        match file_data {
            FileUpload::LastModified(last_modified_time_received) => {
                last_modified_time = last_modified_time_received;
            }
            FileUpload::File(mut file) => {
                let start_time = Instant::now();
                let filename = get_filename(&file);
                let extension = match get_extension(&file) {
                    Ok(ext) => ext,
                    Err(err) => return Err(err),
                };

                warn!(duration = &*format!("{:?}", start_time.elapsed()); "Get filename and extension");
                if VALID_IMAGE_EXTENSIONS.contains(&extension.as_str())
                    || VALID_VIDEO_EXTENSIONS.contains(&extension.as_str())
                {
                    match save_file(&mut file, filename, extension, last_modified_time).await {
                        Ok(final_path) => {
                            let notify = Arc::new(Notify::new());
                            let scan_queue = ScanQueue {
                                scan_list: vec![final_path.into()],
                                notify: Some(notify.clone()),
                            };
                            match EVENTS_SENDER.get().unwrap().send(scan_queue) {
                                Ok(_) => {
                                    // Successfully sent the paths, now wait for scan to complete.
                                    notify.notified().await;
                                }
                                Err(err) => {
                                    // The send failed, and we get `returned_paths` back here.
                                    let error_data = ErrorData::new(
                                        format!("Failed to send paths: {}", err),
                                        format!("Error occurred when sending path: {:?}", err.0),
                                        None,
                                        None,
                                        Location::caller(),
                                        None,
                                    );
                                    handle_error(error_data);
                                }
                            }
                        }
                        Err(err) => return Err(err),
                    }
                } else {
                    error!("Invalid file type");
                    return Err(Status::InternalServerError);
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
) -> Result<String, Status> {
    let unique_id = Uuid::new_v4();
    let path_tmp = format!("./upload/{}-{}.tmp", filename, unique_id);

    match file.move_copy_to(&path_tmp).await {
        Ok(_) => spawn_blocking(move || {
            set_last_modified_time(&path_tmp, last_modified_time)?;
            let path_final = format!("./upload/{}-{}.{}", filename, unique_id, extension);
            match std::fs::rename(&path_tmp, &path_final) {
                Ok(_) => Ok(path_final),
                Err(err) => {
                    error!("Failed to rename file: {}", err);
                    Err(Status::InternalServerError)
                }
            }
        })
        .await
        .unwrap(),
        Err(err) => {
            error!("Failed to save file: {}", err);
            Err(Status::InternalServerError)
        }
    }
}

fn set_last_modified_time(path: &str, last_modified_time: u64) -> Result<(), Status> {
    match filetime::set_file_mtime(
        path,
        filetime::FileTime::from_unix_time((last_modified_time / 1000) as i64, 0),
    ) {
        Ok(_) => Ok(()),
        Err(err) => {
            error!("Failed to set last modified time: {}", err);
            Err(Status::InternalServerError)
        }
    }
}
