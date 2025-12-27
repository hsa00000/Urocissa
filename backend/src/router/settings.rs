use log::error;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{get, post, put};

use crate::public::config::{get_config, update_config};
use crate::public::structure::settings::AppSettings;
use crate::router::fairing::guard_auth::GuardAuth;

// 修改為 /get/settings
#[get("/get/settings")]
pub fn get_settings(_auth: GuardAuth) -> Json<AppSettings> {
    Json(get_config())
}

// 修改為 /put/settings
#[put("/put/settings", data = "<settings>")]
pub fn update_settings(_auth: GuardAuth, settings: Json<AppSettings>) -> Result<Status, Status> {
    match update_config(settings.into_inner()) {
        Ok(_) => Ok(Status::Ok),
        Err(e) => {
            error!("Failed to update settings: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

// 匯出 (下載) - 使用 GET，路徑調整為 /get/settings/export 以符合您的風格
#[get("/get/settings/export")]
pub fn export_settings(_auth: GuardAuth) -> (ContentType, String) {
    let settings = get_config();
    let json = serde_json::to_string_pretty(&settings).unwrap_or_default();
    (ContentType::JSON, json)
}

// 匯入 (上傳) - 使用 POST，路徑調整為 /post/settings/import 以符合您的風格
#[post("/post/settings/import", data = "<file>")]
pub fn import_settings(_auth: GuardAuth, file: Json<AppSettings>) -> Result<Status, Status> {
    // 可以在這裡加一些驗證邏輯
    match update_config(file.into_inner()) {
        Ok(_) => Ok(Status::Ok),
        Err(e) => {
            error!("Import failed: {}", e);
            Err(Status::InternalServerError)
        }
    }
}
