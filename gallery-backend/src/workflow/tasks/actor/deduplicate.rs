use crate::table::relations::database_alias::DatabaseAliasSchema;
use crate::{
    public::{
        constant::DEFAULT_PRIORITY_LIST,
        db::tree::TREE,
        error_data::handle_error,
        structure::{
            abstract_data::AbstractData,
            database::{
                file_modify::FileModify, generate_timestamp::compute_timestamp_ms_by_file_modify,
            },
            guard::PendingGuard,
        },
        tui::DASHBOARD,
    },
    table::{
        image::ImageCombined,
        meta_image::ImageMetadataSchema,
        meta_video::VideoMetadataSchema,
        object::{ObjectSchema, ObjectType},
        video::VideoCombined,
    },
    utils::PathExt,
    workflow::tasks::{
        BATCH_COORDINATOR,
        batcher::flush_tree::{FlushOperation, FlushTreeTask},
    },
};
use anyhow::Result;
use arrayvec::ArrayString;
use mini_executor::Task;
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

pub struct DeduplicateTask {
    pub path: PathBuf,
    pub hash: ArrayString<64>,
    pub presigned_album_id_opt: Option<ArrayString<64>>,
}

impl DeduplicateTask {
    pub fn new(
        path: impl Into<PathBuf>,
        hash: ArrayString<64>,
        presigned_album_id_opt: Option<ArrayString<64>>,
    ) -> Self {
        Self {
            path: path.into(),
            hash,
            presigned_album_id_opt,
        }
    }
}

impl Task for DeduplicateTask {
    // 修改回傳型別，攜帶 FlushOperations 讓後續流程決定何時寫入
    type Output = Result<Option<(AbstractData, Vec<FlushOperation>)>>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            tokio::task::spawn_blocking(move || deduplicate_task(self))
                .await
                .map_err(|err| {
                    handle_error(anyhow::anyhow!("Failed to run deduplicate task: {}", err))
                })?
        }
    }
}

fn deduplicate_task(task: DeduplicateTask) -> Result<Option<(AbstractData, Vec<FlushOperation>)>> {
    let existing_db_opt = TREE.load_data_from_hash(task.hash.as_str())?;

    let metadata = task.path.metadata()?;
    let modified = metadata
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        .duration_since(std::time::SystemTime::UNIX_EPOCH)?
        .as_millis();
    let file_modify = FileModify::new(&task.path, modified);

    // 準備 Flush Operations 列表
    let mut operations = Vec::new();

    // 無論是新檔案還是舊檔案，都需紀錄 Alias
    operations.push(FlushOperation::InsertDatabaseAlias(DatabaseAliasSchema {
        hash: task.hash.as_str().to_string(),
        file: file_modify.file.clone(),
        modified: file_modify.modified as i64,
        scan_time: file_modify.scan_time as i64,
    }));

    match existing_db_opt {
        Some(mut existing_data) => {
            // [Fix] 處理重複檔案：加入相簿關聯
            if let Some(album_id) = task.presigned_album_id_opt {
                match &mut existing_data {
                    AbstractData::Image(i) => {
                        i.albums.insert(album_id);
                    }
                    AbstractData::Video(v) => {
                        v.albums.insert(album_id);
                    }
                    _ => {}
                }
                // 更新該資料的關聯 (FlushTreeTask 會處理 album_database 的同步)
                operations.push(FlushOperation::InsertAbstractData(existing_data.clone()));
            }

            // 執行資料庫寫入
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask { operations });

            // 回傳 None 表示檔案已存在，不需要後續的 Copy/Index 流程
            Ok(None)
        }
        None => {
            // [Fix] 新檔案處理
            let ext = task.path.ext_lower();
            let obj_type = ObjectType::from_ext(&ext).unwrap();

            DASHBOARD.add_task(task.hash, task.path.to_string_lossy().to_string(), obj_type);

            let created_time =
                compute_timestamp_ms_by_file_modify(&file_modify, DEFAULT_PRIORITY_LIST);

            let mut data = match obj_type {
                ObjectType::Image => {
                    let object = ObjectSchema {
                        id: task.hash,
                        created_time,
                        obj_type: "image".to_string(),
                        thumbhash: None,
                        pending: false,
                        tags: HashSet::new(),
                    };
                    let metadata = ImageMetadataSchema {
                        id: task.hash,
                        size: metadata.len(),
                        width: 0,
                        height: 0,
                        ext: ext.clone(),
                        phash: None,
                    };
                    AbstractData::Image(ImageCombined {
                        object,
                        metadata,
                        albums: HashSet::new(),
                        exif_vec: BTreeMap::new(),
                    })
                }
                ObjectType::Video => {
                    let object = ObjectSchema {
                        id: task.hash,
                        created_time,
                        obj_type: "video".to_string(),
                        thumbhash: None,
                        pending: true, // Video starts as pending
                        tags: HashSet::new(),
                    };
                    let metadata = VideoMetadataSchema {
                        id: task.hash,
                        size: 0,
                        width: 0,
                        height: 0,
                        ext: ext.clone(),
                        duration: 0.0,
                    };
                    AbstractData::Video(VideoCombined {
                        object,
                        metadata,
                        albums: HashSet::new(),
                        exif_vec: BTreeMap::new(),
                    })
                }
                ObjectType::Album => unreachable!("Unexpected album type in deduplicate task"),
            };

            // [Fix] 處理新檔案的相簿關聯
            if let Some(album_id) = task.presigned_album_id_opt {
                match &mut data {
                    AbstractData::Image(i) => {
                        i.albums.insert(album_id);
                    }
                    AbstractData::Video(v) => {
                        v.albums.insert(album_id);
                    }
                    _ => {}
                }
            }

            // 注意：對於新檔案，我們回傳 operations，讓 index_workflow 決定何時寫入
            // 這是為了確保 InsertObject (由 index_workflow 產生) 發生在 InsertDatabaseAlias 之前
            Ok(Some((data, operations)))
        }
    }
}
