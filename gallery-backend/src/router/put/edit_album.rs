use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::table::relations::album_database::AlbumItemSchema;
use crate::workflow::processors::transitor::index_to_hash;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::flush_tree::{FlushOperation, FlushTreeTask};
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use arrayvec::ArrayString;
use redb::ReadableTable;
use rocket::serde::{Deserialize, json::Json};
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
                flush_ops.push(FlushOperation::InsertAlbum(AlbumItemSchema {
                    album_id: album_id.to_string(),
                    hash: hash.to_string(),
                }));
            }
            for album_id in &json_data.remove_albums_array {
                flush_ops.push(FlushOperation::RemoveAlbum(AlbumItemSchema {
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
        let _data = data_opt.unwrap();
        let cover_str = cover_hash.as_str();

        // 修正：分別更新 object 與 meta_album
        let tx = TREE.begin_write().unwrap();
        {
            // 1. 更新 meta_album 的 cover
            let album_data = {
                let temp_table = tx.open_table(crate::table::meta_album::META_ALBUM_TABLE).unwrap();
                temp_table.get(&*album_id).unwrap().map(|bytes| bytes.value().to_vec())
            };
            if let Some(album_bytes) = album_data {
                let mut album: crate::table::meta_album::AlbumMetadataSchema = bitcode::decode(&album_bytes).unwrap();
                album.cover = Some(ArrayString::from(cover_str).unwrap());
                let encoded = bitcode::encode(&album);
                let mut table = tx.open_table(crate::table::meta_album::META_ALBUM_TABLE).unwrap();
                table.insert(&*album_id, encoded.as_slice()).unwrap();
            }
        }

        let album_data = TREE.load_data_from_hash(&album_id).unwrap().unwrap();

        // 2. 更新 object 的 thumbhash
        let thumbhash = match &album_data {
            AbstractData::Image(i) => i.object.thumbhash.clone(),
            AbstractData::Video(v) => v.object.thumbhash.clone(),
            AbstractData::Album(_) => None,
        };
        {
            let object_data = {
                let temp_table = tx.open_table(crate::table::object::OBJECT_TABLE).unwrap();
                temp_table.get(&*album_id).unwrap().map(|bytes| bytes.value().to_vec())
            };
            if let Some(object_bytes) = object_data {
                let mut object: crate::table::object::ObjectSchema = bitcode::decode(&object_bytes).unwrap();
                object.thumbhash = thumbhash;
                let encoded = bitcode::encode(&object);
                let mut object_table = tx.open_table(crate::table::object::OBJECT_TABLE).unwrap();
                object_table.insert(&*album_id, encoded.as_slice()).unwrap();
            }
        }

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

        let txn = TREE.begin_write().unwrap();
        {
            let album_data = {
                let temp_table = txn.open_table(crate::table::meta_album::META_ALBUM_TABLE).unwrap();
                temp_table.get(&*album_id).unwrap().map(|bytes| bytes.value().to_vec())
            };
            if let Some(album_bytes) = album_data {
                let mut album: crate::table::meta_album::AlbumMetadataSchema = bitcode::decode(&album_bytes).unwrap();
                album.title = set_album_title_inner.title;
                let encoded = bitcode::encode(&album);
                let mut table = txn.open_table(crate::table::meta_album::META_ALBUM_TABLE).unwrap();
                table.insert(&*album_id, encoded.as_slice()).unwrap();
            }
        }
        txn.commit().unwrap();
    })
    .await
    .unwrap();
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(())
}
