use std::time::Instant;

use arrayvec::ArrayString;
use rand::Rng;
use rand::distr::Alphanumeric;
use redb::ReadableTable;
use rocket::serde::json::Json;
use rocket::{http::Status, post};

use serde::{Deserialize, Serialize};

use crate::public::album::Album;
use crate::public::redb::{ALBUM_TABLE, DATA_TABLE};
use crate::public::tree::TREE;
use crate::public::tree_snapshot::TREE_SNAPSHOT;
use crate::router::fairing::auth_guard::AuthGuard;
use crate::router::fairing::read_only_mod_guard::ReadOnlyModeGuard;
use crate::synchronizer::album::album_self_update_async;

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlbum {
    pub title: Option<String>,
    pub elements_index: Vec<usize>,
    pub timestamp: u128,
}

#[post("/post/create_album", data = "<create_album>")]
pub async fn create_album(
    _auth: AuthGuard,
    _read_only_mode: ReadOnlyModeGuard,
    create_album: Json<CreateAlbum>,
) -> Result<String, Status> {
    let id = tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();
        let create_album = create_album.into_inner();
        let album_id: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            .take(64)
            .map(char::from)
            .collect();
        let album_id = ArrayString::<64>::from(&album_id).unwrap();

        let album_database = Album::new(album_id, create_album.title);
        let txn = TREE.in_disk.begin_write().unwrap();

        let timestamp = &create_album.timestamp;
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(timestamp).unwrap();

        {
            let mut album_table = txn.open_table(ALBUM_TABLE).unwrap();

            album_table
                .insert(album_id.as_str(), &album_database)
                .unwrap();

            let mut data_table = txn.open_table(DATA_TABLE).unwrap();

            create_album.elements_index.iter().for_each(|index| {
                let hash = tree_snapshot.get_hash(*index);

                // album should not be added to album
                let data_opt = data_table.get(hash.as_str()).unwrap().map(|data_guard| {
                    let mut data = data_guard.value();
                    data.album.insert(album_id);
                    data
                });
                if let Some(data) = data_opt {
                    data_table.insert(&*data.hash, &data).unwrap();
                }
            });
        }
        txn.commit().unwrap();
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Create album");
        album_id
    })
    .await
    .unwrap();
    TREE.should_update_async().await;
    album_self_update_async(vec![id]).await;
    Ok(id.to_string())
}
