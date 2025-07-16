use arrayvec::ArrayString;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rocket::http::Status;

use crate::constant::PROCESS_BATCH_NUMBER;
use crate::coordinator::COORDINATOR;
use crate::coordinator::album::AlbumTask;
use crate::db::tree::TREE;
use crate::db::tree_snapshot::TREE_SNAPSHOT;
use crate::indexer::databaser::{regenerate_metadata_for_image, regenerate_metadata_for_video};
use crate::looper::{LOOPER, Signal};
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;

use rocket::serde::json::Json;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateData {
    index_array: Vec<usize>,
    timestamp: u128,
}

#[post("/put/reindex", format = "json", data = "<json_data>")]
pub async fn reindex(
    _auth: GuardAuth,
    _read_only_mode: GuardReadOnlyMode,
    json_data: Json<RegenerateData>,
) -> Status {
    let json_data = json_data.into_inner();
    tokio::task::spawn_blocking(move || {
        let database_table = TREE.api_read_tree();
        let album_table = TREE.api_read_album();
        let reduced_data_vec = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();
        let hash_vec: Vec<ArrayString<64>> = json_data
            .index_array
            .par_iter()
            .map(|index| reduced_data_vec.get_hash(*index))
            .collect();
        let total_batches = (hash_vec.len() + PROCESS_BATCH_NUMBER - 1) / PROCESS_BATCH_NUMBER;

        for (i, batch) in hash_vec.chunks(PROCESS_BATCH_NUMBER).enumerate() {
            info!("Processing batch {}/{}", i + 1, total_batches);

            let list_of_database: Vec<_> = batch
                .into_par_iter()
                .filter_map(|&hash| {
                    match database_table.get(&*hash).unwrap() {
                        Some(guard) => {
                            let mut database = guard.value();
                            if database.ext_type == "image" {
                                match regenerate_metadata_for_image(&mut database) {
                                    Ok(_) => Some(database),
                                    Err(_) => None,
                                }
                            } else if database.ext_type == "video" {
                                match regenerate_metadata_for_video(&mut database) {
                                    Ok(_) => Some(database),
                                    Err(_) => None,
                                }
                            } else {
                                None
                            }
                        }
                        _ => {
                            match album_table.get(&*hash).unwrap() {
                                Some(_) => {
                                    // album_self_update already will commit
                                    COORDINATOR.execute_detached(AlbumTask::new(hash));
                                    None
                                }
                                _ => {
                                    error!(
                                        "Reindex failed: cannot find data with hash/id: {}",
                                        hash
                                    );
                                    None
                                }
                            }
                        }
                    }
                })
                .collect();
            TREE.insert_tree_api(&list_of_database).unwrap();
        }
    })
    .await
    .unwrap();
    LOOPER.notify_with_ack(Signal::UpdateTree).await.unwrap();
    Status::Ok
}
