use crate::public::config::{PUBLIC_CONFIG, PublicConfig};
use crate::public::db::tree::TREE;
use crate::public::db::tree::read_tags::TagInfo;
use crate::table::relations::album_share::{AlbumShareTable, ResolvedShare, Share};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use anyhow::Context;
use arrayvec::ArrayString;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[get("/get/get-config.json")]
pub async fn get_config(auth: GuardResult<GuardShare>) -> AppResult<Json<&'static PublicConfig>> {
    let _ = auth?;
    Ok(Json(&*PUBLIC_CONFIG))
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
        let mut all_shares_map =
            AlbumShareTable::get_all_shares_grouped().context("Failed to fetch shares")?;
        let album_info_list = album_list
            .into_iter()
            .map(|album| {
                let share_list = all_shares_map.remove(album.id.as_str()).unwrap_or_default();
                AlbumInfo {
                    album_id: album.id.to_string(),
                    album_name: album.title,
                    share_list,
                }
            })
            .collect();
        Ok(Json(album_info_list))
    })
    .await?
}

#[get("/get/get-all-shares")]
pub async fn get_all_shares(auth: GuardResult<GuardAuth>) -> AppResult<Json<Vec<ResolvedShare>>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || {
        let shares = AlbumShareTable::get_all_resolved().context("Failed to read all shares")?;
        Ok(Json(shares))
    })
    .await?
}
