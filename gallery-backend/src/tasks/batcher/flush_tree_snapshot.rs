use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use mini_executor::BatchTask;
use std::time::Instant;

use crate::public::error_data::handle_error;
use anyhow;

pub struct FlushTreeSnapshotTask;


impl BatchTask for FlushTreeSnapshotTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            flush_tree_snapshot_task();
        }
    }
}

fn flush_tree_snapshot_task() {
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

            // SQLite Write
            let hashes: Vec<String> = entry_ref.value().iter().map(|d| d.hash.to_string()).collect();
            if let Err(e) = crate::public::db::sqlite::SQLITE.insert_snapshot(timestamp, hashes) {
                 handle_error(anyhow::anyhow!("SQLite insert_snapshot failed: {}", e));
            }

            info!(
                duration = &*format!("{:?}", timer_start.elapsed());
                "Write in-memory cache into disk (SQLite)"
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
}

