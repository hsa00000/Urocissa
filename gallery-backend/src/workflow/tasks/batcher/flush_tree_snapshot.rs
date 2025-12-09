use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use anyhow::Result;
use log::{error, info};
use mini_executor::BatchTask;
use std::time::Instant;

pub struct FlushTreeSnapshotTask;

impl BatchTask for FlushTreeSnapshotTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            if let Err(e) = flush_tree_snapshot_task() {
                error!("Error in flush_tree_snapshot_task: {}", e);
            }
        }
    }
}

fn flush_tree_snapshot_task() -> Result<()> {
    loop {
        if TREE_SNAPSHOT.in_memory.is_empty() {
            break;
        }

        // Narrow scope for the DashMap reference
        let timestamp = {
            // Attempt to get a reference to one entry:
            let Some(entry_ref) = TREE_SNAPSHOT.in_memory.iter().next() else {
                break;
            };

            let timestamp = *entry_ref.key();

            let timer_start = Instant::now();
            
            let txn = TREE_SNAPSHOT.in_disk.begin_write()?;
            {
                let mut table = txn.open_table(crate::public::db::tree_snapshot::new::SNAPSHOTS_TABLE)?;

                for (index, data) in entry_ref.iter().enumerate() {
                    // 使用 bitcode 序列化資料
                    let encoded_data = bitcode::encode(data);
                    table.insert((timestamp, index as u64), encoded_data.as_slice())?;
                }
            }
            txn.commit()?;

            info!(
                duration = &*format!("{:?}", timer_start.elapsed());
                "Write in-memory cache into disk"
            );
            timestamp
        };

        //Remove from DashMap *after* reference is dropped
        TREE_SNAPSHOT.in_memory.remove(&timestamp);
        info!(
            "{} items remaining in in-memory tree cache",
            TREE_SNAPSHOT.in_memory.len()
        );
    }
    Ok(())
}
