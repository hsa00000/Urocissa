use crate::workflow::processors::transitor::index_to_hash;
use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::flush_tree::FlushTreeTask;
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use arrayvec::ArrayString;
use rocket::serde::{Deserialize, json::Json};
use rusqlite::params;
use serde::Serialize;
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditAlbumsData {
    index_array: Vec<usize>,
    add_albums_array: Vec<ArrayString<64>>,
    remove_albums_array: Vec<ArrayString<64>>,
    timestamp: u128,
}

#[put("/put/edit_album", format = "json", data = "<json_data>")]
pub async fn edit_album(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<EditAlbumsData>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;

    // 在 blocking 執行緒產生所有要寫入的 payload 與受影響相簿
    let to_flush = tokio::task::spawn_blocking(move || -> Result<Vec<_>> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        let mut to_flush = Vec::with_capacity(json_data.index_array.len());
        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            let mut database = TREE.load_database_from_hash(&hash)?;
            for album_id in &json_data.add_albums_array {
                database.album.insert(album_id.clone());
            }
            for album_id in &json_data.remove_albums_array {
                database.album.remove(album_id);
            }
            to_flush.push(AbstractData::Database(database));
        }

        Ok(to_flush)
    })
    .await
    .map_err(|e| anyhow::anyhow!("join error: {e}"))??;

    // 單次等待版本
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(to_flush))
        .await?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;

    // 受影響相簿：全部等待
    // Album stats are now updated by database triggers
    // No need to manually update albums

    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumCover {
    pub album_id: ArrayString<64>,
    pub cover_hash: ArrayString<64>,
}

#[put("/put/set_album_cover", data = "<set_album_cover>")]
pub async fn set_album_cover(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    set_album_cover: Json<SetAlbumCover>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || {
        let set_album_cover_inner = set_album_cover.into_inner();
        let album_id = set_album_cover_inner.album_id;
        let cover_hash = set_album_cover_inner.cover_hash;

        let database = TREE.load_database_from_hash(&cover_hash).unwrap();

        // Directly update the album's cover and thumbhash in database
        let cover_str = cover_hash.as_str();
        let conn = TREE.get_connection().unwrap();
        conn.execute(
            "UPDATE album SET cover = ?, thumbhash = ? WHERE id = ?",
            params![cover_str, &database.schema.thumbhash, &*album_id],
        )
        .unwrap();
    })
    .await
    .unwrap();
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumTitle {
    pub album_id: ArrayString<64>,
    pub title: Option<String>,
}

#[put("/put/set_album_title", data = "<set_album_title>")]
pub async fn set_album_title(
    auth: GuardResult<GuardShare>,
    read_only_mode: Result<GuardReadOnlyMode>,
    set_album_title: Json<SetAlbumTitle>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    tokio::task::spawn_blocking(move || {
        let set_album_title_inner = set_album_title.into_inner();
        let album_id = set_album_title_inner.album_id;

        let conn = TREE.get_connection().unwrap();
        // Update the title
        conn.execute(
            "UPDATE album SET title = ? WHERE id = ?",
            [
                set_album_title_inner.title.as_deref().unwrap_or(""),
                &*album_id,
            ],
        )
        .unwrap();
    })
    .await
    .unwrap();
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(())
}
