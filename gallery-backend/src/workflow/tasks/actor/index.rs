use anyhow::{Context, Result};
use arrayvec::ArrayString;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use tokio_rayon::AsyncThreadPool;

use crate::public::constant::runtime::WORKER_RAYON_POOL;
use crate::public::structure::abstract_data::AbstractData;
use crate::table::database::DatabaseSchema;
use crate::table::relations::database_exif::ExifSchema;

use crate::{
    public::{
        constant::VALID_IMAGE_EXTENSIONS,
        error_data::handle_error,
        structure::guard::PendingGuard,
        tui::{DASHBOARD, FileType},
    },
    workflow::info::{process_image_info, process_video_info},
    workflow::tasks::batcher::flush_tree::{FlushOperation, FlushTreeTask},
};
use mini_executor::Task;

#[derive(Debug, Clone)]
pub struct IndexTask {
    pub source_path: PathBuf,
    pub hash: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub thumbhash: Vec<u8>,
    pub phash: Vec<u8>,
    pub ext: String,
    pub ext_type: String,
    pub exif_vec: BTreeMap<String, String>,
    pub timestamp_ms: i64,
    pub pending: bool,
}

impl IndexTask {
    pub fn new(source_path: PathBuf, database: DatabaseSchema) -> Self {
        Self {
            source_path,
            hash: database.hash,
            size: database.size,
            width: database.width,
            height: database.height,
            thumbhash: database.thumbhash,
            phash: database.phash,
            ext: database.ext,
            ext_type: database.ext_type,
            exif_vec: BTreeMap::new(),
            timestamp_ms: database.timestamp_ms,
            pending: false,
        }
    }

    pub fn imported_path_string(&self) -> String {
        format!(
            "./object/imported/{}/{}.{}",
            &self.hash[0..2],
            self.hash,
            self.ext
        )
    }
    pub fn imported_path(&self) -> PathBuf {
        PathBuf::from(self.imported_path_string())
    }
    pub fn compressed_path_string(&self) -> String {
        if self.ext_type == "image" {
            format!("./object/compressed/{}/{}.jpg", &self.hash[0..2], self.hash)
        } else {
            format!("./object/compressed/{}/{}.mp4", &self.hash[0..2], self.hash)
        }
    }
    pub fn compressed_path(&self) -> PathBuf {
        PathBuf::from(self.compressed_path_string())
    }
    pub fn thumbnail_path(&self) -> String {
        format!("./object/compressed/{}/{}.jpg", &self.hash[0..2], self.hash)
    }
    pub fn compressed_path_parent(&self) -> PathBuf {
        self.compressed_path()
            .parent()
            .expect("Path::new(&output_file_path_string).parent() fail")
            .to_path_buf()
    }
}

impl Task for IndexTask {
    type Output = Result<(IndexTask, FlushTreeTask)>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            WORKER_RAYON_POOL
                .spawn_async(move || index_task_match(self))
                .await
                .map_err(|err| handle_error(err.context("Failed to run index task")))
        }
    }
}

/// Outer layer: unify business result matching and update TUI  
/// (success -> advance, failure -> mark_failed)
fn index_task_match(index_task: IndexTask) -> Result<(IndexTask, FlushTreeTask)> {
    let hash = index_task.hash; // hash is Copy, no need to clone
    match index_task_result(index_task) {
        Ok((index_task, task)) => {
            DASHBOARD.advance_task_state(&hash);
            Ok((index_task, task))
        }
        Err(e) => {
            DASHBOARD.mark_failed(&hash);
            Err(e)
        }
    }
}

/// Inner layer: only responsible for business logic, no TUI state updates
fn index_task_result(mut index_task: IndexTask) -> Result<(IndexTask, FlushTreeTask)> {
    let hash = index_task.hash;
    let newest_path = index_task.source_path.to_string_lossy().to_string();
    // Register task in dashboard; attach context if extension is invalid
    DASHBOARD.add_task(
        hash,
        newest_path.clone(),
        FileType::try_from(index_task.ext_type.as_str())
            .context(format!("unsupported file type: {}", index_task.ext))?,
    );

    // Branch processing based on file extension
    let is_image = VALID_IMAGE_EXTENSIONS.contains(&index_task.ext.as_str());
    if is_image {
        process_image_info(&mut index_task).context(format!(
            "failed to process image metadata pipeline:\n{:#?}",
            index_task
        ))?;
    } else {
        process_video_info(&mut index_task).context(format!(
            "failed to process video metadata pipeline:\n{:#?}",
            index_task
        ))?;
        index_task.pending = true;
    };

    let abstract_data = AbstractData::DatabaseSchema(index_task.clone().into());
    let mut operations = vec![FlushOperation::InsertAbstractData(abstract_data)];

    // Insert EXIF data
    let hash_str = index_task.hash.as_str();
    for (tag, value) in &index_task.exif_vec {
        operations.push(FlushOperation::InsertExif(ExifSchema {
            hash: hash_str.to_string(),
            tag: tag.clone(),
            value: value.clone(),
        }));
    }

    let flush_task = FlushTreeTask { operations };

    Ok((index_task, flush_task))
}

impl From<IndexTask> for DatabaseSchema {
    fn from(task: IndexTask) -> Self {
        Self {
            hash: task.hash,
            size: task.size,
            width: task.width,
            height: task.height,
            thumbhash: task.thumbhash,
            phash: task.phash,
            ext: task.ext,
            album: HashSet::new(),
            ext_type: task.ext_type,
            pending: task.pending,
            timestamp_ms: task.timestamp_ms,
        }
    }
}
