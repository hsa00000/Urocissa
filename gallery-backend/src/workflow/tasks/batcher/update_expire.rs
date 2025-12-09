use crate::public::db::query_snapshot::QUERY_SNAPSHOT;
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::workflow::processors::transitor::get_current_timestamp_u64;
use anyhow::Result;
use log::{error, info};
use mini_executor::BatchTask;
use redb::ReadableTable;
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
        let txn = QUERY_SNAPSHOT.in_disk.begin_write()?;
        let (deleted, marked) = {
            // 1. 【機會主義清理】先刪除那些「已經過期」的舊垃圾
            let mut data_table = txn.open_table(crate::public::db::query_snapshot::QUERY_SNAPSHOT_TABLE)?;
            let mut expiry_table = txn.open_table(crate::public::db::query_snapshot::QUERY_EXPIRY_TABLE)?;
            
            // 清理過期的快照
            let mut deleted = 0;
            let mut to_delete = Vec::new();
            for item in expiry_table.range(..=(current_timestamp, u64::MAX))? {
                let (key, _) = item?;
                let (expires_at, query_hash) = key.value();
                if expires_at < current_timestamp {
                    data_table.remove(query_hash)?;
                    to_delete.push((expires_at, query_hash));
                    deleted += 1;
                }
            }
            for key in to_delete {
                expiry_table.remove(key)?;
            }

            // 2. 【軟過期】將原本「永久有效」的舊快照，標記為 1 小時後過期
            let mut marked = 0;
            for item in data_table.iter()? {
                let (query_hash, _) = item?;
                let query_hash_val = query_hash.value();
                // 檢查是否已經有過期時間
                if expiry_table.get(&(expire_time, query_hash_val)).is_err() {
                    // 如果沒有過期記錄，添加一個
                    expiry_table.insert((expire_time, query_hash_val), ())?;
                    marked += 1;
                }
            }
            (deleted, marked)
        };
        txn.commit()?;

        info!(
            "Version updated. Cleaned {} old snapshots. Marked {} snapshots to expire at {}",
            deleted, marked, expire_time
        );
    }
    Ok(())
}
