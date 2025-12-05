use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::table::relations::album_database::AlbumDatabaseSchema;
use crate::workflow::processors::transitor::index_to_hash;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::flush_tree::{FlushOperation, FlushTreeTask};
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
    let flush_ops = tokio::task::spawn_blocking(move || -> Result<Vec<FlushOperation>> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        let mut flush_ops = Vec::new();
        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            for album_id in &json_data.add_albums_array {
                flush_ops.push(FlushOperation::InsertAlbum(AlbumDatabaseSchema {
                    album_id: album_id.to_string(),
                    hash: hash.to_string(),
                }));
            }
            for album_id in &json_data.remove_albums_array {
                flush_ops.push(FlushOperation::RemoveAlbum(AlbumDatabaseSchema {
                    album_id: album_id.to_string(),
                    hash: hash.to_string(),
                }));
            }
        }

        Ok(flush_ops)
    })
    .await
    .map_err(|e| anyhow::anyhow!("join error: {e}"))??;

    // 單次等待版本
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask {
            operations: flush_ops,
        })
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

        let data_opt = TREE.load_data_from_hash(&cover_hash).unwrap();
        let data = data_opt.unwrap();
        let cover_str = cover_hash.as_str();

        // 修正：分別更新 object 與 meta_album
        let mut conn = TREE.get_connection().unwrap();
        let tx = conn.transaction().unwrap();

        // 1. 更新 meta_album 的 cover
        tx.execute(
            "UPDATE meta_album SET cover = ? WHERE id = ?",
            params![cover_str, &*album_id],
        )
        .unwrap();

        // 2. 更新 object 的 thumbhash
        let thumbhash = match &data {
            AbstractData::Image(i) => i.object.thumbhash.clone(),
            AbstractData::Video(v) => v.object.thumbhash.clone(),
            AbstractData::Album(_) => None,
        };
        tx.execute(
            "UPDATE object SET thumbhash = ? WHERE id = ?",
            params![&thumbhash, &*album_id],
        )
        .unwrap();

        tx.commit().unwrap();
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
        // 修正：更新 meta_album 表
        conn.execute(
            "UPDATE meta_album SET title = ? WHERE id = ?",
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
