use crate::database::ops::snapshot::query::QUERY_SNAPSHOT;
use anyhow::Result;
use log::{error, info};
use mini_executor::BatchTask;
use std::time::Instant;

pub struct FlushQuerySnapshotTask;

impl BatchTask for FlushQuerySnapshotTask {
    fn batch_run(_: Vec<Self>) -> impl std::future::Future<Output = ()> + Send {
        async move {
            if let Err(e) = flush_query_snapshot_task() {
                error!("Error in flush_query_snapshot_task: {}", e);
            }
        }
    }
}

fn flush_query_snapshot_task() -> Result<()> {
    loop {
        if QUERY_SNAPSHOT.in_memory.is_empty() {
            break;
        }

        // 取得一個 entry 準備寫入
        let expression_hashed = {
            let Some(entry_ref) = QUERY_SNAPSHOT.in_memory.iter().next() else {
                break;
            };

            let expression_hashed = *entry_ref.key();
            let ref_data = entry_ref.value();

            let timer_start = Instant::now();
            let txn = QUERY_SNAPSHOT.in_disk.begin_write()?;

            // 序列化資料
            let data = bitcode::encode(ref_data);

            // 寫入 Redb
            // 預設 expires_at 為 NULL，代表這是當前版本的有效快照
            {
                let mut data_table = txn.open_table(crate::database::ops::snapshot::query::QUERY_SNAPSHOT_TABLE)?;
                data_table.insert(expression_hashed, data.as_slice())?;
            }
            txn.commit()?;

            info!(
                duration = &*format!("{:?}", timer_start.elapsed());
                "Write query cache into disk"
            );

            expression_hashed
        };

        // 從記憶體移除 (Write Buffer 行為)
        // 註：這會清空記憶體，但讀取時我們會透過 read_query_snapshot 自動回補 (Promote)
        QUERY_SNAPSHOT.in_memory.remove(&expression_hashed);

        info!(
            "{} items remaining in in-memory query cache",
            QUERY_SNAPSHOT.in_memory.len()
        );
    }
    Ok(())
}
