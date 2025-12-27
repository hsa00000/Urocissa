use log::error;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{get, post, put};

use crate::public::config::update_config;
use crate::public::structure::config::{AppConfig, APP_CONFIG};
use crate::router::fairing::guard_auth::GuardAuth;

// Refactor: Route path changed to /get/config
#[get("/get/config")]
pub fn get_config_handler(_auth: GuardAuth) -> Json<AppConfig> {
    Json(APP_CONFIG.get().unwrap().read().unwrap().clone())
}

// Refactor: Route path changed to /put/config
#[put("/put/config", data = "<config>")]
pub fn update_config_handler(_auth: GuardAuth, config: Json<AppConfig>) -> Result<Status, Status> {
    match update_config(config.into_inner()) {
        Ok(_) => Ok(Status::Ok),
        Err(e) => {
            error!("Failed to update config: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

// Refactor: Route path changed to /get/config/export
#[get("/get/config/export")]
pub fn export_config_handler(_auth: GuardAuth) -> (ContentType, String) {
    let config = APP_CONFIG.get().unwrap().read().unwrap();
    let json = serde_json::to_string_pretty(&*config).unwrap_or_default();
    (ContentType::JSON, json)
}

// Refactor: Route path changed to /post/config/import
#[post("/post/config/import", data = "<file>")]
pub fn import_config_handler(_auth: GuardAuth, file: Json<AppConfig>) -> Result<Status, Status> {
    match update_config(file.into_inner()) {
        Ok(_) => Ok(Status::Ok),
        Err(e) => {
            error!("Import failed: {}", e);
            Err(Status::InternalServerError)
        }
    }
}
