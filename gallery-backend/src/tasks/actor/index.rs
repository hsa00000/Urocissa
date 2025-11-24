use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use arrayvec::ArrayString;
use std::path::{Path, PathBuf};
use tokio_rayon::AsyncThreadPool;

use crate::public::constant::runtime::{BATCH_RUNTIME, WORKER_RAYON_POOL};
use crate::public::structure::abstract_data::AbstractData;
use crate::tasks::BATCH_COORDINATOR;

use crate::{
    process::info::{process_image_info, process_video_info},
    public::{
        constant::VALID_IMAGE_EXTENSIONS,
        error_data::handle_error,
        structure::{database_struct::database::definition::DatabaseSchema, guard::PendingGuard},
        tui::{DASHBOARD, FileType},
    },
    tasks::batcher::flush_tree::FlushTreeTask,
};
use mini_executor::Task;

pub struct IndexTask {
    pub path: PathBuf,
    pub hash: ArrayString<64>,
}

impl IndexTask {
    pub fn new(path: PathBuf, hash: ArrayString<64>) -> Self {
        Self { path, hash }
    }
}

impl Task for IndexTask {
    type Output = Result<DatabaseSchema>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            let database = DatabaseSchema::new(&self.path, self.hash)?;
            WORKER_RAYON_POOL
                .spawn_async(move || index_task_match(database, &self.path))
                .await
                .map_err(|err| handle_error(err.context("Failed to run index task")))
        }
    }
}

/// Outer layer: unify business result matching and update TUI  
/// (success -> advance, failure -> mark_failed)
fn index_task_match(database: DatabaseSchema, path: &Path) -> Result<DatabaseSchema> {
    let hash = database.hash; // hash is Copy, no need to clone
    match index_task(database, path) {
        Ok(db) => {
            DASHBOARD.advance_task_state(&hash);
            Ok(db)
        }
        Err(e) => {
            DASHBOARD.mark_failed(&hash);
            Err(e)
        }
    }
}

/// Inner layer: only responsible for business logic, no TUI state updates
fn index_task(mut database: DatabaseSchema, path: &Path) -> Result<DatabaseSchema> {
    let hash = database.hash;
    let newest_path = path.to_string_lossy().to_string();

    // Register task in dashboard; attach context if extension is invalid
    DASHBOARD.add_task(
        hash,
        newest_path.clone(),
        FileType::try_from(database.ext_type.as_str())
            .context(format!("unsupported file type: {}", database.ext_type))?,
    );

    // Branch processing based on file extension
    let is_image = VALID_IMAGE_EXTENSIONS.contains(&database.ext.as_str());
    if is_image {
        process_image_info(&mut database).context(format!(
            "failed to process image metadata pipeline:\n{:#?}",
            database
        ))?;
    } else {
        process_video_info(&mut database).context(format!(
            "failed to process video metadata pipeline:\n{:#?}",
            database
        ))?;
        database.pending = true;
    }

    let abstract_data = AbstractData::DatabaseSchema(database.clone().into());
    BATCH_RUNTIME.block_on(async {
        BATCH_COORDINATOR
            .execute_batch_waiting(FlushTreeTask::insert(vec![abstract_data]))
            .await
    })?;

    todo!("接下來要更新 database_alias 表格");

    Ok(database)
}
