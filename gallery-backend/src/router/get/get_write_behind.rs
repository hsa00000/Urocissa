use rocket::serde::json::Json;

use crate::public::db::write_behind::{WRITE_BEHIND, WriteBehindStatus};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::{AppResult, GuardResult};

#[get("/get/write-behind-status")]
pub fn get_write_behind_status(auth: GuardResult<GuardAuth>) -> AppResult<Json<WriteBehindStatus>> {
    let _ = auth?;
    Ok(Json(WRITE_BEHIND.status()))
}
