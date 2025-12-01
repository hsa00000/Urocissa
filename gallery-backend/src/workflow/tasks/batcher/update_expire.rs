use crate::public::db::query_snapshot::QUERY_SNAPSHOT;
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::public::db::types::SqliteU64;
use crate::workflow::processors::transitor::get_current_timestamp_u64;
use anyhow::Result;
use log::{error, info};
use mini_executor::BatchTask;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub struct UpdateExpireTask;

impl BatchTask for UpdateExpireTask {
    fn batch_run(_: Vec<Self>) -> impl std::future::Future<Output = ()> + Send {
        async move {
            if let Err(e) = update_expire_task() {
                error!("Error in update_expire_task: {}", e);
            }
        }
    }
}

fn update_expire_task() -> Result<()> {
    let current_timestamp = get_current_timestamp_u64();
    // 檢查版本號是否變更
    let last_timestamp = VERSION_COUNT_TIMESTAMP.swap(current_timestamp, Ordering::SeqCst);

    if last_timestamp > 0 {
        // 設定 1 小時後過期 (Grace Period)
        let expire_time = current_timestamp + Duration::from_secs(60 * 60).as_millis() as u64;
        let conn = QUERY_SNAPSHOT.in_disk.get()?;

        // 1. 【機會主義清理】先刪除那些「已經過期」的舊垃圾
        let deleted = conn.execute(
            "DELETE FROM query_snapshot WHERE expires_at IS NOT NULL AND expires_at < ?",
            [SqliteU64(current_timestamp)],
        )?;

        // 2. 【軟過期】將原本「永久有效 (expires_at IS NULL)」的舊快照，標記為 1 小時後過期
        let marked = conn.execute(
            "UPDATE query_snapshot SET expires_at = ? WHERE expires_at IS NULL",
            [SqliteU64(expire_time)],
        )?;

        info!(
            "Version updated. Cleaned {} old snapshots. Marked {} snapshots to expire at {}",
            deleted, marked, expire_time
        );

        // 這裡不需要再觸發 ExpireCheckTask，因為我們已經順手清理了
    }
    Ok(())
}
