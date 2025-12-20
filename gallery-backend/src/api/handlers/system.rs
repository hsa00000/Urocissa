use anyhow::Result;
use arrayvec::ArrayString;
use log::{error, info};
use rand::Rng;
use rand::seq::IndexedRandom;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post, put};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use crate::api::fairings::guards::auth::GuardAuth;
use crate::api::fairings::guards::readonly::GuardReadOnlyMode;
use crate::api::{AppResult, GuardResult};
use crate::background::actors::BATCH_COORDINATOR;
use crate::background::actors::indexer::IndexTask;
use crate::background::batchers::flush_tree::{FlushOperation, FlushTreeTask};
use crate::background::batchers::update_tree::UpdateTreeTask;
use crate::background::processors::image::regenerate_metadata_for_image;
use crate::background::processors::transitor::index_to_hash;
use crate::background::processors::video::{regenerate_metadata_for_video, video_duration};
use crate::common::PROCESS_BATCH_NUMBER;
use crate::database::ops::snapshot::tree::TREE_SNAPSHOT;
use crate::database::ops::tree::TREE;
use crate::database::schema::image::ImageCombined;
use crate::database::schema::meta_image::ImageMetadataSchema;
use crate::database::schema::object::{ObjectSchema, ObjectType};
use crate::database::schema::relations::exif::ExifSchema;
use crate::database::schema::relations::tag::TagDatabaseSchema;
use crate::models::entity::abstract_data::AbstractData;
use crate::utils::imported_path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditTagsData {
    index_array: Vec<usize>,
    add_tags_array: Vec<String>,
    remove_tags_array: Vec<String>,
    timestamp: u128,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditStatusData {
    index_array: Vec<usize>,
    timestamp: u128,
    #[serde(default)]
    is_favorite: Option<bool>,
    #[serde(default)]
    is_archived: Option<bool>,
    #[serde(default)]
    is_trashed: Option<bool>,
}

#[put("/put/edit_status", format = "json", data = "<json_data>")]
pub async fn edit_status(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<EditStatusData>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let flush_ops = tokio::task::spawn_blocking(move || -> Result<Vec<FlushOperation>> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        let mut flush_ops = Vec::new();
        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            let mut abstract_data = TREE.load_from_db(&hash)?;

            let object = match &mut abstract_data {
                AbstractData::Image(i) => &mut i.object,
                AbstractData::Video(v) => &mut v.object,
                AbstractData::Album(a) => &mut a.object,
            };

            let mut modified = false;
            if let Some(value) = json_data.is_favorite {
                if object.is_favorite != value {
                    object.is_favorite = value;
                    modified = true;
                }
            }
            if let Some(value) = json_data.is_archived {
                if object.is_archived != value {
                    object.is_archived = value;
                    modified = true;
                }
            }
            if let Some(value) = json_data.is_trashed {
                if object.is_trashed != value {
                    object.is_trashed = value;
                    modified = true;
                }
            }

            if modified {
                flush_ops.push(FlushOperation::InsertAbstractData(abstract_data));
            }
        }

        Ok(flush_ops)
    })
    .await
    .unwrap()?;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask {
            operations: flush_ops,
        })
        .await
        .unwrap();

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(())
}

#[put("/put/edit_tag", format = "json", data = "<json_data>")]
pub async fn edit_tag(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    json_data: Json<EditTagsData>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let flush_ops = tokio::task::spawn_blocking(move || -> Result<Vec<FlushOperation>> {
        let tree_snapshot = TREE_SNAPSHOT
            .read_tree_snapshot(&json_data.timestamp)
            .unwrap();

        let mut flush_ops = Vec::new();
        for &index in &json_data.index_array {
            let hash = index_to_hash(&tree_snapshot, index)?;
            let abstract_data = TREE.load_from_db(&hash)?;

            let object_id = match &abstract_data {
                AbstractData::Image(i) => i.object.id.to_string(),
                AbstractData::Video(v) => v.object.id.to_string(),
                AbstractData::Album(a) => a.object.id.to_string(),
            };

            for tag in &json_data.add_tags_array {
                flush_ops.push(FlushOperation::InsertTag(TagDatabaseSchema {
                    hash: object_id.clone(),
                    tag: tag.clone(),
                }));
            }
            for tag in &json_data.remove_tags_array {
                flush_ops.push(FlushOperation::RemoveTag(TagDatabaseSchema {
                    hash: object_id.clone(),
                    tag: tag.clone(),
                }));
            }
        }

        Ok(flush_ops)
    })
    .await
    .unwrap()?;

    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask {
            operations: flush_ops,
        })
        .await
        .unwrap();

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    Ok(())
}

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
                            let mut abstract_data = AbstractData::Image(img);
                            // [FIX] Capture EXIF data
                            match regenerate_metadata_for_image(&mut abstract_data) {
                                Ok(exif_vec) => {
                                    let mut ops =
                                        vec![FlushOperation::InsertAbstractData(abstract_data)];

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
                                }
                                Err(e) => {
                                    error!("Failed to regenerate image {}: {}", hash, e);
                                    None
                                }
                            }
                        }
                        Some(AbstractData::Video(vid)) => {
                            let mut abstract_data = AbstractData::Video(vid);
                            let imported_path = if let AbstractData::Video(ref v) = abstract_data {
                                imported_path(&v.object.id, &v.metadata.ext)
                            } else {
                                PathBuf::new()
                            };
                            let mut index_task =
                                IndexTask::new(imported_path.clone(), abstract_data.clone());

                            match regenerate_metadata_for_video(&mut index_task) {
                                Ok(_) => {
                                    let duration = video_duration(&imported_path.to_string_lossy())
                                        .unwrap_or(0.0);

                                    if let AbstractData::Video(ref mut vid) = abstract_data {
                                        vid.metadata.width = index_task.width;
                                        vid.metadata.height = index_task.height;
                                        vid.metadata.size = index_task.size;
                                        vid.object.thumbhash = Some(index_task.thumbhash);
                                        vid.metadata.duration = duration;

                                        let mut ops =
                                            vec![FlushOperation::InsertAbstractData(abstract_data)];

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
fn create_random_data() -> AbstractData {
    // 取得隨機數產生器 (rand 0.9+)
    let mut rng = rand::rng();

    // 1. 隨機長寬
    let short_side = rng.random_range(600..1080);
    let long_side = rng.random_range(1080..1920);

    let (width, height) = if rng.random_bool(0.5) {
        (long_side, short_side)
    } else {
        (short_side, long_side)
    };

    // 2. 隨機 Tags
    let tag_pool = vec![
        "Nature",
        "Urban",
        "Travel",
        "Food",
        "Family",
        "Pets",
        "Summer",
        "Winter",
        "Screenshot",
        "Memes",
    ];
    let num_tags = rng.random_range(0..=3);
    let mut tags = HashSet::new();
    // 使用 IndexedRandom::choose_multiple
    let chosen_tags: Vec<_> = tag_pool
        .choose_multiple(&mut rng, num_tags)
        .cloned()
        .collect();
    for tag in chosen_tags {
        tags.insert(tag.to_string());
    }

    // 3. 隨機 Description
    let desc_pool = vec![
        "A beautiful memory.",
        "Taken with my phone.",
        "I love this place!",
        "Random shot.",
        "Testing data generation.",
        "Unbelievable view!",
    ];
    let description = if rng.random_bool(0.6) {
        // 使用 IndexedRandom::choose
        desc_pool.choose(&mut rng).map(|s| s.to_string())
    } else {
        None
    };

    // ID
    let id_raw = format!("random_{}", rng.random::<u64>());
    let id = ArrayString::from(&id_raw).unwrap_or_default();

    let image = ImageCombined {
        object: ObjectSchema {
            id,
            obj_type: ObjectType::Image,
            created_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            pending: false,
            thumbhash: None,
            description,
            tags,
            is_favorite: rng.random_bool(0.2),
            is_archived: rng.random_bool(0.1),
            is_trashed: false,
        },
        metadata: ImageMetadataSchema {
            id,
            size: rng.random_range(50_000..5_000_000),
            width,
            height,
            ext: "jpg".to_string(),
            phash: None,
        },
        albums: HashSet::new(),
        exif_vec: BTreeMap::new(),
    };

    AbstractData::Image(image)
}
#[get("/put/generate_random_data?<number>")]
pub async fn generate_random_data(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    number: usize,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let database_list: Vec<AbstractData> = (0..number)
        .into_par_iter()
        .map(|_| create_random_data())
        .collect();
    BATCH_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(database_list))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to flush tree: {}", e))?;
    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update tree: {}", e))?;
    info!("Insert random data complete");
    Ok(())
}

pub fn generate_system_routes() -> Vec<rocket::Route> {
    routes![edit_tag, edit_status, reindex, generate_random_data]
}
