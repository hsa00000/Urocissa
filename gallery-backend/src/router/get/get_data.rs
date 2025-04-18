use crate::public::album::Share;
use crate::public::config::{PUBLIC_CONFIG, PublicConfig};
use crate::public::tree::TREE;
use crate::public::tree::read_tags::TagInfo;
use crate::router::fairing::guard_auth::{GuardAuth, GuardAuthEdit};

use arrayvec::ArrayString;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[get("/get/get-config.json")]
pub async fn get_config(_auth: GuardAuth) -> Json<&'static PublicConfig> {
    Json(&*PUBLIC_CONFIG)
}

#[get("/get/get-tags")]
pub async fn get_tags(_auth: GuardAuthEdit) -> Json<Vec<TagInfo>> {
    tokio::task::spawn_blocking(move || {
        let vec_tags_info = TREE.read_tags();
        Json(vec_tags_info)
    })
    .await
    .unwrap()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumInfo {
    pub album_id: String,
    pub album_name: Option<String>,
    pub share_list: HashMap<ArrayString<64>, Share>,
}

#[get("/get/get-albums")]
pub async fn get_albums(_auth: GuardAuthEdit) -> Json<Vec<AlbumInfo>> {
    tokio::task::spawn_blocking(move || {
        let album_list = TREE.read_albums();
        let album_info_list = album_list
            .into_iter()
            .map(|album| AlbumInfo {
                album_id: album.id.to_string(),
                album_name: album.title,
                share_list: album.share_list,
            })
            .collect();
        Json(album_info_list)
    })
    .await
    .unwrap()
}
