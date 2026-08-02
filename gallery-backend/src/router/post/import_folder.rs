use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use serde::Deserialize;

use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::{AppResult, GuardResult};
use crate::tasks::actor::folder_import::{cancel_folder_import, start_folder_import};

#[derive(Deserialize)]
pub struct StartFolderImportRequest {
    path: String,
}

#[post("/post/import/folder", data = "<req>")]
#[allow(clippy::needless_pass_by_value)]
pub fn start_folder_import_handler(
    _auth: GuardAuth,
    read_only: GuardResult<GuardReadOnlyMode>,
    req: Json<StartFolderImportRequest>,
) -> AppResult<Status> {
    let _ = read_only?;
    start_folder_import(&req.path)?;
    Ok(Status::Accepted)
}

#[post("/post/import/folder/cancel")]
pub fn cancel_folder_import_handler(_auth: GuardAuth) -> AppResult<Status> {
    cancel_folder_import()?;
    Ok(Status::Ok)
}
