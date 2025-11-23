use crate::{
    operations::indexation::generate_compressed_video::generate_compressed_video,
    public::{
        constant::runtime::WORKER_RAYON_POOL,
        error_data::handle_error,
        structure::{
            abstract_data::AbstractData,
            database_struct::database::definition::{Database, DatabaseWithTag},
            guard::PendingGuard,
        },
        tui::DASHBOARD,
    },
    tasks::{BATCH_COORDINATOR, batcher::flush_tree::FlushTreeTask},
};
use anyhow::Context;
use anyhow::Result;
use mini_executor::Task;
use tokio_rayon::AsyncThreadPool;

pub struct VideoTask {
    database: DatabaseWithTag,
}

impl VideoTask {
    pub fn new(database: DatabaseWithTag) -> Self {
        Self { database }
    }
}

impl Task for VideoTask {
    type Output = Result<()>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            WORKER_RAYON_POOL
                .spawn_async(move || video_task(self.database))
                .await
                .map_err(|err| handle_error(err.context("Failed to run video task")))
        }
    }
}

pub fn video_task(mut database: DatabaseWithTag) -> Result<()> {
    let hash = database.hash;
    let mut db = Database::from(database);
    match generate_compressed_video(&mut db) {
        Ok(_) => {
            database = DatabaseWithTag::from(db);
            database.pending = false;
            let abstract_data = AbstractData::Database(database.clone());
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]));

            DASHBOARD.advance_task_state(&hash);
        }
        Err(err) => Err(err).context(format!(
            "video_task: video compression failed for hash: {}",
            hash
        ))?,
    }
    Ok(())
}
