use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use log::debug;
use tokio_rayon::AsyncThreadPool;

use crate::public::constant::runtime::WORKER_RAYON_POOL;
use crate::tasks::BATCH_COORDINATOR;

use crate::{
    process::info::{process_image_info, process_video_info},
    public::{
        error_data::handle_error,
        structure::{abstract_data::AbstractData, guard::PendingGuard},
        tui::{DASHBOARD, FileType},
    },
    tasks::batcher::flush_tree::FlushTreeTask,
};
use mini_executor::Task;

pub struct IndexTask {
    pub abstract_data: AbstractData,
}

impl IndexTask {
    pub fn new(abstract_data: AbstractData) -> Self {
        Self { abstract_data }
    }
}

impl Task for IndexTask {
    type Output = Result<AbstractData>;

    async fn run(self) -> Self::Output {
        let _pending_guard = PendingGuard::new();
        let abstract_data = WORKER_RAYON_POOL
            .spawn_async(move || index_task_match(self.abstract_data))
            .await
            .map_err(|err| handle_error(err.context("Failed to run index task")))?;
        BATCH_COORDINATOR
            .execute_batch_waiting(FlushTreeTask::insert(vec![abstract_data.clone()]))
            .await
            .map_err(|err| {
                handle_error(anyhow::Error::new(err).context("Failed to persist indexed media"))
            })?;
        Ok(abstract_data)
    }
}

/// Outer layer: unify business result matching and update TUI\
/// (success -> advance, failure -> `mark_failed`)
fn index_task_match(abstract_data: AbstractData) -> Result<AbstractData> {
    let hash = abstract_data.hash();
    match index_task(abstract_data) {
        Ok(data) => {
            DASHBOARD.advance_task_state(&hash);
            Ok(data)
        }
        Err(e) => {
            DASHBOARD.mark_failed(&hash);
            Err(e)
        }
    }
}

/// Inner layer: only responsible for business logic, no TUI state updates
fn index_task(mut abstract_data: AbstractData) -> Result<AbstractData> {
    let hash = abstract_data.hash();
    let newest_path = abstract_data
        .alias()
        .iter()
        .max()
        .ok_or_else(|| anyhow!("alias collection is empty for hash: {hash}"))?
        .file
        .clone();

    // Register task in dashboard; attach context if extension is invalid
    DASHBOARD.add_task(
        hash,
        newest_path.clone(),
        FileType::try_from(abstract_data.ext_type()).context(format!(
            "unsupported file type: {}",
            abstract_data.ext_type()
        ))?,
    );

    // Branch processing based on file type
    let is_image = abstract_data.is_image();
    if is_image {
        if let Err(e) = process_image_info(&mut abstract_data) {
            debug!("Failed image data dump: {abstract_data:#?}");
            return Err(e).context(format!(
                "failed to process image metadata pipeline. Hash: {}, Path: {}",
                abstract_data.hash(),
                newest_path
            ));
        }
    } else {
        if let Err(e) = process_video_info(&mut abstract_data) {
            debug!("Failed video data dump: {abstract_data:#?}");
            return Err(e).context(format!(
                "failed to process video metadata pipeline. Hash: {}, Path: {}",
                abstract_data.hash(),
                newest_path
            ));
        }
        abstract_data.set_pending(true);
    }

    Ok(abstract_data)
}
