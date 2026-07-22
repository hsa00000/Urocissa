use crate::public::db::tree::TREE;
use crate::public::db::tree::read_tags::SearchFacets;
use crate::public::error::AppError; // Import AppError
use crate::public::structure::album::Share;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::{AppResult, GuardResult};
use arrayvec::ArrayString;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[get("/get/get-search-facets")]
pub async fn get_search_facets(auth: GuardResult<GuardAuth>) -> AppResult<Json<SearchFacets>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || -> AppResult<Json<SearchFacets>> {
        let start_time = Instant::now();
        let facets = TREE.read_search_facets();
        crate::perf_timing!(
            "get_list.read_search_facets_response",
            start_time,
            "Create search facets response"
        );
        Ok(Json(facets))
    })
    .await
    .map_err(|e| AppError::from(anyhow::Error::from(e)))? // Handle JoinError
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
    tokio::task::spawn_blocking(move || -> AppResult<Json<Vec<AlbumInfo>>> {
        let start_time = Instant::now();
        let album_list = TREE
            .read_albums()
            .map_err(|e| e.context("Failed to read albums"))?;
        let album_info_list: Vec<AlbumInfo> = album_list
            .into_iter()
            .map(|album| AlbumInfo {
                album_id: album.object.id.to_string(),
                album_name: album.metadata.title,
                share_list: album.metadata.share_list,
            })
            .collect();
        crate::perf_timing!(
            "get_list.read_albums_response",
            start_time,
            "Create albums response: {} items",
            album_info_list.len()
        );
        Ok(Json(album_info_list))
    })
    .await
    .map_err(|e| AppError::from(anyhow::Error::from(e)))?
}
