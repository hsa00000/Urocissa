use crate::{
    operations::indexation::generate_compressed_video::generate_compressed_video,
    public::{
        constant::runtime::WORKER_RAYON_POOL,
        error_data::handle_error,
        structure::{
            abstract_data::AbstractData,
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
    abstract_data: AbstractData,
}

impl VideoTask {
    pub fn new(abstract_data: AbstractData) -> Self {
        Self { abstract_data }
    }
}

impl Task for VideoTask {
    type Output = Result<()>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            WORKER_RAYON_POOL
                .spawn_async(move || video_task(self.abstract_data))
                .await
                .map_err(|err| handle_error(err.context("Failed to run video task")))
        }
    }
}

pub fn video_task(mut abstract_data: AbstractData) -> Result<()> {
    let hash = abstract_data.hash();
    match generate_compressed_video(&mut abstract_data) {
        Ok(_) => {
            abstract_data.set_pending(false);
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data.clone()]));

            DASHBOARD.advance_task_state(&hash);
        }
        Err(err) => Err(err).context(format!(
            "video_task: video compression failed for hash: {}",
            hash
        ))?,
    }
    Ok(())
}
