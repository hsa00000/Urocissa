use crate::public::constant::PROCESS_BATCH_NUMBER;
use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::{AbstractData, Database};
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::fairing::guard_auth::GuardAuth;
use crate::router::fairing::guard_read_only_mode::GuardReadOnlyMode;
use crate::table::relations::database_exif::ExifSchema; // [FIX] Import ExifSchema
use crate::workflow::processors::image::regenerate_metadata_for_image;
use crate::workflow::processors::video::{regenerate_metadata_for_video, video_duration};
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::actor::index::IndexTask;
use crate::workflow::tasks::batcher::flush_tree::{FlushOperation, FlushTreeTask}; // [FIX] Import FlushOperation
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;
use anyhow::Result;
use arrayvec::ArrayString;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rocket::http::Status;
use rocket::serde::json::Json;
use serde::Deserialize;
use std::collections::HashSet;

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

            // [FIX] Collect Vec<FlushOperation> instead of Vec<AbstractData>
            let operations_list: Vec<Vec<FlushOperation>> = batch
                .into_par_iter()
                .filter_map(|&hash| {
                    let abstract_data_opt = TREE.load_from_db(&hash).ok();
                    match abstract_data_opt {
                        Some(AbstractData::Image(img)) => {
                            // 創建 Database 結構體來調用現有函數
                            let mut db = Database {
                                media: AbstractData::Image(img),
                                album: HashSet::new(),
                            };
                            // [FIX] Capture EXIF data
                            match regenerate_metadata_for_image(&mut db) {
                                Ok(exif_vec) => {
                                    if let AbstractData::Image(updated_img) = db.media {
                                        let mut ops = vec![FlushOperation::InsertAbstractData(
                                            AbstractData::Image(updated_img),
                                        )];

                                        // [FIX] Add InsertExif operations
                                        let hash_str = hash.as_str();
                                        for (tag, value) in exif_vec {
                                            ops.push(FlushOperation::InsertExif(ExifSchema {
                                                hash: hash_str.to_string(),
                                                tag,
                                                value,
                                            }));
                                        }
                                        Some(ops)
                                    } else {
                                        None
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to regenerate image {}: {}", hash, e);
                                    None
                                }
                            }
                        }
                        Some(AbstractData::Video(vid)) => {
                            // [FIX] Implement Video logic similar to Image/Database
                            let mut db = Database {
                                media: AbstractData::Video(vid),
                                album: HashSet::new(),
                            };
                            let mut index_task =
                                IndexTask::new(db.imported_path(), db.media.clone());

                            match regenerate_metadata_for_video(&mut index_task) {
                                Ok(_) => {
                                    let duration =
                                        video_duration(&db.imported_path_string()).unwrap_or(0.0);

                                    if let AbstractData::Video(ref mut vid) = db.media {
                                        vid.metadata.width = index_task.width;
                                        vid.metadata.height = index_task.height;
                                        vid.metadata.size = index_task.size;
                                        vid.object.thumbhash = Some(index_task.thumbhash);
                                        vid.metadata.duration = duration;

                                        let mut ops = vec![FlushOperation::InsertAbstractData(
                                            AbstractData::Video(vid.clone()),
                                        )];

                                        // [FIX] Add InsertExif operations from index_task
                                        let hash_str = hash.as_str();
                                        for (tag, value) in index_task.exif_vec {
                                            ops.push(FlushOperation::InsertExif(ExifSchema {
                                                hash: hash_str.to_string(),
                                                tag,
                                                value,
                                            }));
                                        }
                                        Some(ops)
                                    } else {
                                        None
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to regenerate video {}: {}", hash, e);
                                    None
                                }
                            }
                        }
                        Some(AbstractData::Album(_)) => None,
                        None => {
                            error!("Reindex failed: cannot find data with hash/id: {}", hash);
                            None
                        }
                    }
                })
                .collect();

            // [FIX] Flatten the list of operations and execute
            let all_ops: Vec<FlushOperation> = operations_list.into_iter().flatten().collect();
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask {
                operations: all_ops,
            });
        }
    })
    .await
    .unwrap();
    BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
    Ok(Status::Ok)
}
