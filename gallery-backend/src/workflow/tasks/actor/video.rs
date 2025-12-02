use crate::{
    workflow::processors::video::generate_compressed_video,
    public::{
        constant::runtime::WORKER_RAYON_POOL,
        error_data::handle_error,
        structure::{
            abstract_data::AbstractData,
            guard::PendingGuard,
        },
        tui::DASHBOARD,
    },
    table::database::DatabaseSchema,
    workflow::tasks::{BATCH_COORDINATOR, batcher::flush_tree::FlushTreeTask},
};
use anyhow::Context;
use anyhow::Result;
use mini_executor::Task;
use tokio_rayon::AsyncThreadPool;

pub struct VideoTask {
    database: DatabaseSchema,
}

impl VideoTask {
    pub fn new(database: DatabaseSchema) -> Self {
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

pub fn video_task(mut database: DatabaseSchema) -> Result<()> {
    let hash = database.hash;
    match generate_compressed_video(&mut database) {
        Ok(_) => {
            database.pending = false;
            let abstract_data = AbstractData::Database(crate::public::structure::abstract_data::Database {
                schema: database.clone(),
                album: std::collections::HashSet::new(),
            });
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
