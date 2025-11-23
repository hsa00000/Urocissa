use arrayvec::ArrayString;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rocket::http::Status;

use crate::process::info::regenerate_metadata_for_image;
use crate::process::info::regenerate_metadata_for_video;
use crate::public::constant::PROCESS_BATCH_NUMBER;
use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::database_struct::database::definition::{Database, DatabaseWithTag};
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::INDEX_COORDINATOR;
use crate::tasks::actor::album::AlbumSelfUpdateTask;
use crate::tasks::batcher::flush_tree::FlushTreeTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
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
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<RegenerateData>,
) -> AppResult<Status> {
    let _ = auth?;
    let _ = read_only_mode?;
    let json_data = json_data.into_inner();
    tokio::task::spawn_blocking(move || {
        let reduced_data_vec = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();
        let hash_vec: Vec<ArrayString<64>> = json_data
            .index_array
            .par_iter()
            .map(|index| reduced_data_vec.get_hash(*index).unwrap())
            .collect();
        let total_batches = (hash_vec.len() + PROCESS_BATCH_NUMBER - 1) / PROCESS_BATCH_NUMBER;

        for (i, batch) in hash_vec.chunks(PROCESS_BATCH_NUMBER).enumerate() {
            info!("Processing batch {}/{}", i + 1, total_batches);

            let database_list: Vec<_> = batch
                .into_par_iter()
                .filter_map(|&hash| {
                    let abstract_data_opt = TREE.load_from_db(&hash).ok();
                    match abstract_data_opt {
                        Some(AbstractData::Database(database)) => {
                            if database.ext_type == "image" {
                                let mut db = Database::from(database);
                                match regenerate_metadata_for_image(&mut db) {
                                    Ok(_) => Some(AbstractData::Database(DatabaseWithTag::from(db).into())),
                                    Err(_) => None,
                                }
                            } else if database.ext_type == "video" {
                                let mut db = Database::from(database);
                                match regenerate_metadata_for_video(&mut db) {
                                    Ok(_) => Some(AbstractData::Database(DatabaseWithTag::from(db).into())),
                                    Err(_) => None,
                                }
                            } else {
                                None
                            }
                        }
                        Some(AbstractData::Album(_)) => {
                            // album_self_update already will commit
                            INDEX_COORDINATOR.execute_detached(AlbumSelfUpdateTask::new(hash));
                            None
                        }
                        None => {
                            error!("Reindex failed: cannot find data with hash/id: {}", hash);
                            None
                        }
                    }
                })
                .collect();
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(database_list));
        }
    })
    .await
    .unwrap();
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
    Ok(Status::Ok)
}
