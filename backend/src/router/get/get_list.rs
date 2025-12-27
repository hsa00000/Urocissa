use crate::public::structure::config::APP_CONFIG;
use crate::public::db::tree::TREE;
use crate::public::db::tree::read_tags::TagInfo;
use crate::public::structure::album::Share;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use anyhow::Context;
use arrayvec::ArrayString;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicConfigResponse {
    pub read_only_mode: bool,
    pub disable_img: bool,
}

#[get("/get/get-config.json")]
pub fn get_public_config(auth: GuardResult<GuardShare>) -> AppResult<Json<PublicConfigResponse>> {
    let _ = auth?;
    let config = APP_CONFIG.get().unwrap().read().unwrap();
    Ok(Json(PublicConfigResponse {
        read_only_mode: config.read_only_mode,
        disable_img: config.disable_img,
    }))
}

#[get("/get/get-tags")]
pub async fn get_tags(auth: GuardResult<GuardAuth>) -> AppResult<Json<Vec<TagInfo>>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || {
        let vec_tags_info = TREE.read_tags();
        Ok(Json(vec_tags_info))
    })
    .await?
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumInfo {
    pub album_id: String,
    pub album_name: Option<String>,
    pub share_list: HashMap<ArrayString<64>, Share>,
}

#[get("/get/get-albums")]
pub async fn get_albums(auth: GuardResult<GuardAuth>) -> AppResult<Json<Vec<AlbumInfo>>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || {
        let album_list = TREE.read_albums().context("Failed to read albums")?;
        let album_info_list = album_list
            .into_iter()
            .map(|album| AlbumInfo {
                album_id: album.object.id.to_string(),
                album_name: album.metadata.title,
                share_list: album.metadata.share_list,
            })
            .collect();
        Ok(Json(album_info_list))
    })
    .await?
}
