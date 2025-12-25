use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::process::transitor::index_to_abstract_data;
use crate::public::constant::redb::DATA_TABLE;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use crate::router::{AppResult, GuardResult};
use crate::tasks::actor::album::AlbumSelfUpdateTask;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::tasks::{BATCH_COORDINATOR, INDEX_COORDINATOR};
use anyhow::Result;
use arrayvec::ArrayString;
use futures::{StreamExt, TryStreamExt, stream};
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
    let (to_flush, effected_album_vec) =
        tokio::task::spawn_blocking(move || -> Result<(Vec<_>, Vec<ArrayString<64>>)> {
            let tree_snapshot = open_tree_snapshot_table(json_data.timestamp)?;
            let data_table = open_data_table();

            let mut to_flush = Vec::with_capacity(json_data.index_array.len());
            for &index in &json_data.index_array {
                let mut abstract_data =
                    index_to_abstract_data(&tree_snapshot, &data_table, index)?;
                if let Some(albums) = abstract_data.albums_mut() {
                    for album_id in &json_data.add_albums_array {
                        albums.insert(album_id.clone());
                    }
                    for album_id in &json_data.remove_albums_array {
                        albums.remove(album_id);
                    }
                }
                to_flush.push(abstract_data);
            }

            let effected_album_vec = json_data
                .add_albums_array
                .iter()
                .chain(json_data.remove_albums_array.iter())
                .cloned()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            Ok((to_flush, effected_album_vec))
        })
        .await
        .map_err(|e| anyhow::anyhow!("join error: {e}"))??;

    // 單次等待版本（取代 detached 逐筆呼叫）
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(to_flush))
        .await?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;

    // 受影響相簿：全部等待（有界並行）
    const ALBUM_CONC: usize = 8;
    stream::iter(effected_album_vec)
        .map(|album_id| async move {
            INDEX_COORDINATOR
                .execute_waiting(AlbumSelfUpdateTask::new(album_id))
                .await
        })
        .buffer_unordered(ALBUM_CONC)
        .try_collect::<Vec<_>>()
        .await?;

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

        let txn = TREE.in_disk.begin_write().unwrap();
        {
            let mut data_table = txn.open_table(DATA_TABLE).unwrap();

            let album = data_table.get(&*album_id).unwrap().unwrap().value();
            let mut album = match album {
                AbstractData::Album(album) => album,
                _ => panic!("Expected Album but got different type"),
            };
            let database = data_table.get(&*cover_hash).unwrap().unwrap().value();

            album.set_cover(&database);
            data_table.insert(&*album_id, AbstractData::Album(album)).unwrap();
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

        let txn = TREE.in_disk.begin_write().unwrap();
        {
            let mut data_table = txn.open_table(DATA_TABLE).unwrap();

            let album = data_table.get(&*album_id).unwrap().unwrap().value();
            let mut album = match album {
                AbstractData::Album(album) => album,
                _ => panic!("Expected Album but got different type"),
            };

            album.metadata.title = set_album_title_inner.title;
            data_table.insert(&*album_id, AbstractData::Album(album)).unwrap();
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
