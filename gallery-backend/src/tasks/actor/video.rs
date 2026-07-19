use crate::{
    process::{
        artifact_publisher::ArtifactPublisher,
        media_pipeline::{
            MediaTaskPlan, ReindexOperation, execute_media_pipeline_with_video_progress,
        },
    },
    public::{
        constant::runtime::WORKER_RAYON_POOL,
        error_data::handle_error,
        structure::{abstract_data::AbstractData, guard::PendingGuard},
        tui::DASHBOARD,
    },
    tasks::{BATCH_COORDINATOR, batcher::flush_tree::FlushTreeTask},
};
use anyhow::Context;
use anyhow::Result;
use mini_executor::Task;
use tokio_rayon::AsyncThreadPool;
use uuid::Uuid;

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

    async fn run(self) -> Self::Output {
        let _pending_guard = PendingGuard::new();
        WORKER_RAYON_POOL
            .spawn_async(move || video_task(self.abstract_data))
            .await
            .map_err(|err| handle_error(err.context("Failed to run video task")))
    }
}

pub fn video_task(mut abstract_data: AbstractData) -> Result<()> {
    let hash = abstract_data.hash();
    let plan = MediaTaskPlan::new(vec![ReindexOperation::VideoCompression])?;
    let mut publisher = ArtifactPublisher::new(format!("import-video-{}", Uuid::new_v4()));
    match execute_media_pipeline_with_video_progress(&abstract_data, &plan, &mut publisher, true) {
        Ok(result) => {
            publisher.publish(|| Ok(()))?;
            abstract_data = result.candidate;
            abstract_data.set_pending(false);
            BATCH_COORDINATOR
                .execute_batch_detached(FlushTreeTask::insert(vec![abstract_data.clone()]));

            DASHBOARD.advance_task_state(&hash);
        }
        Err(err) => Err(err).context(format!(
            "video_task: video compression failed for hash: {hash}"
        ))?,
    }
    Ok(())
}
