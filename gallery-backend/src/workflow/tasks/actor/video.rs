use crate::{
    workflow::processors::video::{generate_compressed_video, VideoProcessResult},
    public::{
        constant::runtime::WORKER_RAYON_POOL,
        error_data::handle_error,
        structure::{
            abstract_data::{AbstractData, Database},
            guard::PendingGuard,
        },
        tui::DASHBOARD,
    },
    workflow::tasks::{BATCH_COORDINATOR, batcher::flush_tree::FlushTreeTask},
};
use anyhow::Context;
use anyhow::Result;
use log::info;
use mini_executor::Task;
use tokio_rayon::AsyncThreadPool;

pub struct VideoTask {
    database: Database,
}

impl VideoTask {
    pub fn new(database: Database) -> Self {
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

pub fn video_task(mut database: Database) -> Result<()> {
    let hash = database.hash();
    match generate_compressed_video(&mut database) {
        Ok(VideoProcessResult::Success) => {
            database.set_pending(false);
            let abstract_data = AbstractData::Database(database);
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]));

            DASHBOARD.advance_task_state(&hash);
        }
        Ok(VideoProcessResult::ConvertedToImage) => {
            // 轉換為圖片後，我們也需要 Flush 更新資料庫的 obj_type
            // 視需求決定是否要將 pending 設為 false，或者讓它進入 Image 的處理流程
            database.set_pending(false); 
            let abstract_data = AbstractData::Database(database);
            BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask::insert(vec![abstract_data]));
            
            // 可以在 Dashboard 顯示為完成，或記錄日誌
            info!("Video task: Converted {} to image", hash);
            DASHBOARD.advance_task_state(&hash);
        }
        Err(err) => Err(err).context(format!(
            "video_task: video compression failed for hash: {}",
            hash
        ))?,
    }
    Ok(())
}
