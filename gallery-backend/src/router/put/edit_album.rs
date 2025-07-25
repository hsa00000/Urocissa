use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::process::transitor::index_to_database;
use crate::public::constant::redb::{ALBUM_TABLE, DATA_TABLE};
use crate::public::db::tree::TREE;
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
use futures::future::join_all;
use redb::ReadableTable;
use rocket::serde::{Deserialize, json::Json};
use serde::Serialize;
use std::collections::HashSet;
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
    let effected_album_vec = tokio::task::spawn_blocking(move || -> Result<_> {
        let tree_snapshot = open_tree_snapshot_table(json_data.timestamp)?;
        let data_table = open_data_table()?;

        for &index in &json_data.index_array {
            let mut database = index_to_database(&tree_snapshot, &data_table, index)?;
            for album_id in &json_data.add_albums_array {
                database.album.insert(album_id.clone());
            }
            for album_id in &json_data.remove_albums_array {
                database.album.remove(album_id);
            }
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![database.into()]));
        }

        let effected_album_vec: Vec<ArrayString<64>> = json_data
            .add_albums_array
            .iter()
            .chain(json_data.remove_albums_array.iter())
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        Ok(effected_album_vec)
    })
    .await
    .unwrap()?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await?;
    let futures = effected_album_vec.into_iter().map(async |album_id| {
        INDEX_COORDINATOR
            .execute_waiting(AlbumSelfUpdateTask::new(album_id))
            .await
    });
    join_all(futures).await;
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetAlbumCover {
    pub album_id: ArrayString<64>,
    pub cover_hash: ArrayString<64>,
}

#[post("/post/set_album_cover", data = "<set_album_cover>")]
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
            let mut album_table = txn.open_table(ALBUM_TABLE).unwrap();
            let data_table = txn.open_table(DATA_TABLE).unwrap();

            let mut album = album_table.get(&*album_id).unwrap().unwrap().value();
            let database = data_table.get(&*cover_hash).unwrap().unwrap().value();

            album.set_cover(&database);
            album_table.insert(&*album_id, album).unwrap();
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

#[post("/post/set_album_title", data = "<set_album_title>")]
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
            let mut album_table = txn.open_table(ALBUM_TABLE).unwrap();

            let mut album = album_table.get(&*album_id).unwrap().unwrap().value();

            album.title = set_album_title_inner.title;
            album_table.insert(&*album_id, album).unwrap();
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
