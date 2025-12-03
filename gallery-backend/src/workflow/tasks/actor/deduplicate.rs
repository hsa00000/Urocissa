use crate::{
    public::{
        constant::DEFAULT_PRIORITY_LIST,
        db::tree::TREE,
        error_data::handle_error,
        structure::{
            abstract_data::{AbstractData, Database, MediaWithAlbum},
            database::{
                file_modify::FileModify, generate_timestamp::compute_timestamp_ms_by_file_modify,
            },
        },
        tui::DASHBOARD,
    },
    table::{
        image::ImageCombined,
        meta_image::ImageMetadataSchema,
        meta_video::VideoMetadataSchema,
        object::{ObjectSchema, ObjectType},
        relations::database_alias::DatabaseAliasSchema,
        video::VideoCombined,
    },
    workflow::tasks::{
        BATCH_COORDINATOR,
        batcher::flush_tree::{FlushOperation, FlushTreeTask},
    },
};
use anyhow::Result;
use arrayvec::ArrayString;
use mini_executor::Task;
use std::{collections::HashSet, path::PathBuf, time::SystemTime};
use tokio::task::spawn_blocking;

pub struct DeduplicateTask {
    pub path: PathBuf,
    pub hash: ArrayString<64>,
    pub presigned_album_id_opt: Option<ArrayString<64>>,
}

impl DeduplicateTask {
    pub fn new(
        path: PathBuf,
        hash: ArrayString<64>,
        presigned_album_id_opt: Option<ArrayString<64>>,
    ) -> Self {
        Self {
            path,
            hash,
            presigned_album_id_opt,
        }
    }
}

impl Task for DeduplicateTask {
    type Output = Result<Option<(Database, FlushTreeTask)>>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || deduplicate_task(self))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run deduplicate task")))
        }
    }
}

fn deduplicate_task(task: DeduplicateTask) -> Result<Option<(Database, FlushTreeTask)>> {
    let existing_db_opt = TREE.load_database_from_hash(task.hash.as_str())?;

    let metadata = task.path.metadata()?;
    let modified = metadata
        .modified()?
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();
    let file_modify = FileModify::new(&task.path, modified);

    match existing_db_opt {
        Some(mut database_exist) => {
            let mut operations = Vec::new();
            operations.push(FlushOperation::InsertDatabaseAlias(DatabaseAliasSchema {
                hash: database_exist.hash().as_str().to_string(),
                file: file_modify.file,
                modified: file_modify.modified as i64,
                scan_time: file_modify.scan_time as i64,
            }));

            if let Some(album_id) = task.presigned_album_id_opt {
                database_exist.album.insert(album_id);
                operations.push(FlushOperation::InsertAbstractData(AbstractData::Database(
                    database_exist.clone(),
                )));
                BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask { operations });
            }

            Ok(None)
        }
        None => {
            // new files

            let ext = task
                .path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            let ext_type = ObjectType::str_from_ext(&ext);

            // Register to Dashboard (Task Start)
            let obj_type = ObjectType::from_str(ext_type).unwrap_or(ObjectType::Image);
            DASHBOARD.add_task(task.hash, task.path.to_string_lossy().to_string(), obj_type);

            let created_time =
                compute_timestamp_ms_by_file_modify(&file_modify, DEFAULT_PRIORITY_LIST);

            let media = match obj_type {
                ObjectType::Image => {
                    let object = ObjectSchema {
                        id: task.hash,
                        created_time,
                        obj_type: "image".to_string(),
                        thumbhash: None,
                        pending: false,
                    };
                    let metadata = ImageMetadataSchema {
                        id: task.hash,
                        size: metadata.len(),
                        width: 0,
                        height: 0,
                        ext: ext.clone(),
                        phash: None,
                    };
                    MediaWithAlbum::Image(ImageCombined { object, metadata })
                }
                ObjectType::Video => {
                    let object = ObjectSchema {
                        id: task.hash,
                        created_time,
                        obj_type: "video".to_string(),
                        thumbhash: None,
                        pending: true, // Video starts as pending until processed
                    };
                    let metadata = VideoMetadataSchema {
                        id: task.hash,
                        size: metadata.len(),
                        width: 0,
                        height: 0,
                        ext: ext.clone(),
                        duration: 0.0,
                    };
                    MediaWithAlbum::Video(VideoCombined { object, metadata })
                }
                ObjectType::Album => {
                    // 不應該發生
                    panic!("Unexpected album type in deduplicate task");
                }
            };

            let mut database = Database {
                media,
                album: HashSet::new(),
            };

            if let Some(album_id) = task.presigned_album_id_opt {
                database.album.insert(album_id);
            }

            // Prepare Flush Operations (Alias only for now)
            let mut operations = Vec::new();
            operations.push(FlushOperation::InsertDatabaseAlias(DatabaseAliasSchema {
                hash: task.hash.as_str().to_string(),
                file: file_modify.file,
                modified: file_modify.modified as i64,
                scan_time: file_modify.scan_time as i64,
            }));

            // We pass the database structure to the next task (Copy -> Index)
            // IndexTask will be responsible for filling in the metadata and triggering the final DB update
            Ok(Some((database, FlushTreeTask { operations })))
        }
    }
}
