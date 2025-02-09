use rocket::serde::json::Json;
use rocket::{http::Status, post};

use crate::router::fairing::{AuthGuard, ReadOnlyModeGuard};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateShare {
    pub album_id: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: u64,
}

#[post("/post/create_share", data = "<create_share>")]
pub async fn create_share(
    _auth: AuthGuard,
    _read_only_mode: ReadOnlyModeGuard,
    create_share: Json<CreateShare>,
) -> Result<String, Status> {
    todo!();
}
