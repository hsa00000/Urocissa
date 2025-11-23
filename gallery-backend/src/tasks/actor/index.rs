use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use tokio_rayon::AsyncThreadPool;

use crate::public::constant::runtime::WORKER_RAYON_POOL;
use crate::public::structure::abstract_data::AbstractData;
use crate::tasks::BATCH_COORDINATOR;

use crate::{
    process::info::{process_image_info, process_video_info},
    public::{
        constant::VALID_IMAGE_EXTENSIONS,
        error_data::handle_error,
        structure::{
            database_struct::database::definition::{Database, DatabaseWithTag},
            guard::PendingGuard,
        },
        tui::{DASHBOARD, FileType},
    },
    tasks::batcher::flush_tree::FlushTreeTask,
};
use mini_executor::Task;

pub struct IndexTask {
    pub database: DatabaseWithTag,
}

impl IndexTask {
    pub fn new(database: DatabaseWithTag) -> Self {
        Self { database }
    }
}

impl Task for IndexTask {
    type Output = Result<DatabaseWithTag>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            WORKER_RAYON_POOL
                .spawn_async(move || index_task_match(self.database))
                .await
                .map_err(|err| handle_error(err.context("Failed to run index task")))
        }
    }
}

/// Outer layer: unify business result matching and update TUI  
/// (success -> advance, failure -> mark_failed)
fn index_task_match(database: DatabaseWithTag) -> Result<DatabaseWithTag> {
    let hash = database.hash; // hash is Copy, no need to clone
    match index_task(database) {
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
fn index_task(mut database: DatabaseWithTag) -> Result<DatabaseWithTag> {
    let hash = database.hash;
    let newest_path = database
        .alias
        .iter()
        .max()
        .ok_or_else(|| anyhow!("alias collection is empty for hash: {}", hash))?
        .file
        .clone();

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
        let mut db = Database::from(database);
        process_image_info(&mut db).context(format!(
            "failed to process image metadata pipeline:\n{:#?}",
            db
        ))?;
        database = DatabaseWithTag::from(db);
    } else {
        let mut db = Database::from(database);
        process_video_info(&mut db).context(format!(
            "failed to process video metadata pipeline:\n{:#?}",
            db
        ))?;
        database = DatabaseWithTag::from(db);
        database.pending = true;
    }

    let abstract_data = AbstractData::Database(database.clone().into());
    BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]));

    Ok(database)
}
