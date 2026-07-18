// src/router/post/import_config.rs

use log::error;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::public::error::{AppError, ErrorKind};
use crate::public::structure::config::AppConfig;
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;

#[post("/post/config/import", data = "<file>")]
pub fn import_config_handler(_auth: GuardAuth, file: Json<AppConfig>) -> AppResult<Status> {
    let config = file.into_inner();
    config
        .public
        .write_behind
        .validate()
        .map_err(|error| AppError::from_err(ErrorKind::InvalidInput, error))?;
    match AppConfig::update(config) {
        Ok(()) => Ok(Status::Ok),
        Err(e) => {
            error!("Import failed: {e}");
            Err(AppError::from_err(ErrorKind::Internal, e))
        }
    }
}
