use crate::{
    common::{
        consts::runtime::WORKER_RAYON_POOL,
        errors::handle_error,
    },
    models::{
        entity::abstract_data::AbstractData,
        dto::guards::PendingGuard,
    },
    cli::tui::DASHBOARD,
    background::{
        processors::video::{VideoProcessResult, generate_compressed_video},
        actors::BATCH_COORDINATOR,
        batchers::flush_tree::FlushTreeTask,
    },
};
use anyhow::{Context, Result};
use log::info;
use mini_executor::Task;
use tokio_rayon::AsyncThreadPool;

pub struct VideoTask {
    data: AbstractData,
}

impl VideoTask {
    pub fn new(data: AbstractData) -> Self {
        Self { data }
    }
}

impl Task for VideoTask {
    type Output = Result<()>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _pending_guard = PendingGuard::new();
            WORKER_RAYON_POOL
                .spawn_async(move || video_task(self.data))
                .await
                .map_err(|err| handle_error(err.context("Failed to run video task")))
        }
    }
}

pub fn video_task(mut data: AbstractData) -> Result<()> {
    let hash = data.hash();
    match generate_compressed_video(&mut data) {
        Ok(VideoProcessResult::Success) => {
            // 壓縮成功，解除 pending 狀態
            if let AbstractData::Video(ref mut vid) = data {
                vid.object.pending = false;
            }

            // 更新資料庫
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![data]));

            DASHBOARD.advance_task_state(&hash);
        }
        Ok(VideoProcessResult::ConvertedToImage) => {
            // 轉換為圖片後，data 已經變為 AbstractData::Image
            // 我們需要 Flush 更新資料庫的 obj_type
            if let AbstractData::Image(ref mut img) = data {
                img.object.pending = false;
            }

            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![data]));

            // 可以在 Dashboard 顯示為完成，或記錄日誌
            info!("Video task: Converted {} to image", hash);
            DASHBOARD.advance_task_state(&hash);
        }
        Err(err) => {
            // 通知 Dashboard 任務失敗
            DASHBOARD.mark_failed(&hash);

            Err(err).context(format!(
                "video_task: video compression failed for hash: {}",
                hash
            ))?
        }
    }
    Ok(())
}
