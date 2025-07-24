use crate::tasks::actor::album::AlbumSelfUpdateTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::tasks::{BATCH_COORDINATOR, INDEX_COORDINATOR};

use crate::public::db::{tree::TREE, tree_snapshot::TREE_SNAPSHOT};
use crate::router::AppResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::router::fairing::guard_share::GuardShare;
use anyhow::Result;

use std::collections::HashSet;

use crate::public::constant::redb::{ALBUM_TABLE, DATA_TABLE};

use arrayvec::ArrayString;
use futures::future::join_all;
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
    auth: Result<GuardAuth>,
    _read_only_mode: GuardReadOnlyMode,
    json_data: Json<EditAlbumsData>,
) -> AppResult<()> {
    let _ = auth?;
    let concact_result = tokio::task::spawn_blocking(move || {
        let txn = TREE.in_disk.begin_write().unwrap();
        {
            let mut write_table = txn.open_table(DATA_TABLE).unwrap();

            let timestamp = &json_data.timestamp;
            let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(timestamp).unwrap();

            json_data.index_array.iter().for_each(|index| {
                let hash = tree_snapshot.get_hash(*index).unwrap();
                let data_opt = match write_table.get(hash.as_str()).unwrap() {
                    Some(guard) => {
                        let mut data = guard.value();
                        json_data.add_albums_array.iter().for_each(|album_id| {
                            data.album.insert(album_id.clone());
                        });

                        json_data.remove_albums_array.iter().for_each(|album_id| {
                            data.album.remove(album_id);
                        });
                        Some(data)
                    }
                    None => None,
                };
                if let Some(data) = data_opt {
                    write_table.insert(&*data.hash, &data).unwrap();
                };
            });
        }
        txn.commit().unwrap();

        let concact_result: Vec<ArrayString<64>> = json_data
            .add_albums_array
            .iter()
            .chain(json_data.remove_albums_array.iter())
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        concact_result
    })
    .await
    .unwrap();

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();
    let futures = concact_result.into_iter().map(async |album_id| {
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
    auth: Result<GuardAuth>,
    _read_only_mode: GuardReadOnlyMode,
    set_album_cover: Json<SetAlbumCover>,
) -> AppResult<()> {
    let _ = auth?;
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
    auth: Result<GuardShare>,
    _read_only_mode: GuardReadOnlyMode,
    set_album_title: Json<SetAlbumTitle>,
) -> AppResult<()> {
    let _ = auth?;
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
