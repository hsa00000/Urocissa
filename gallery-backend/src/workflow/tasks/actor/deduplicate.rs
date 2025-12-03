use crate::{
public::{
constant::{DEFAULT_PRIORITY_LIST, VALID_IMAGE_EXTENSIONS},
db::tree::TREE,
error_data::handle_error,
structure::{
abstract_data::{AbstractData, Database, MediaWithAlbum},
database::{
file_modify::FileModify,
generate_timestamp::compute_timestamp_ms_by_file_modify,
},
},
tui::{DASHBOARD, FileType},
},
table::{
image::ImageCombined,
meta_image::ImageMetadataSchema,
meta_video::VideoMetadataSchema,
object::ObjectSchema,
relations::database_alias::DatabaseAliasSchema,
video::VideoCombined,
},
workflow::{
tasks::{
batcher::flush_tree::{FlushOperation, FlushTreeTask},
BATCH_COORDINATOR,
},
},
};
use anyhow::Result;
use arrayvec::ArrayString;
use mini_executor::Task;
use std::{
collections::HashSet,
path::PathBuf,
time::SystemTime,
};
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

fn deduplicate_task(
task: DeduplicateTask,
) -> Result<Option<(Database, FlushTreeTask)>> {
// Try to load existing database from TREE
let existing_db_result = TREE.load_database_from_hash(task.hash.as_str());

let metadata = task.path.metadata()?;
let modified = metadata
    .modified()?
    .duration_since(SystemTime::UNIX_EPOCH)?
    .as_millis();
let file_modify = FileModify::new(&task.path, modified);

match existing_db_result {
    Ok(mut database_exist) => {
        let mut operations = Vec::new();
        operations.push(FlushOperation::InsertDatabaseAlias(DatabaseAliasSchema {
            hash: database_exist.hash().as_str().to_string(),
            file: file_modify.file,
            modified: file_modify.modified as i64,
            scan_time: file_modify.scan_time as i64,
        }));

        if let Some(album_id) = task.presigned_album_id_opt {
            database_exist.album.insert(album_id);
        }
        
        // For existing files, we might need to update AbstractData if we modified the album set
        operations.push(FlushOperation::InsertAbstractData(AbstractData::Database(
            database_exist.clone(),
        )));

        BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask { operations });
        Ok(None)
    }
    Err(_) => {
        // For new files:
        // 1. Determine type
        // 2. Create a basic Database structure (with 0 width/height, empty hashes)
        // 3. Return it so the flow can pass it to CopyTask and IndexTask
        
        let ext = task
            .path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let ext_type = if VALID_IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            "image"
        } else {
            "video"
        };

        // Register to Dashboard (Task Start)
        DASHBOARD.add_task(
            task.hash,
            task.path.to_string_lossy().to_string(),
            FileType::try_from(ext_type).unwrap_or(FileType::Image),
        );

        // Create timestamp
        let created_time = compute_timestamp_ms_by_file_modify(&file_modify, DEFAULT_PRIORITY_LIST);

        // Construct minimal MediaWithAlbum
        // Note: Actual metadata (width, height, hashes) will be populated by IndexTask later
        let media = if ext_type == "image" {
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
        } else {
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
