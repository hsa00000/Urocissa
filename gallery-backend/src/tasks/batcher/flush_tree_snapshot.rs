use crate::public::db::tree_snapshot::{
    SCROLLBAR_METADATA_TABLE, TREE_SNAPSHOT, TREE_SNAPSHOT_TABLE,
};
use crate::storage::codec;
use mini_executor::BatchTask;
use std::time::Instant;

use crate::public::error_data::handle_error;
use anyhow;

pub struct FlushTreeSnapshotTask;

impl BatchTask for FlushTreeSnapshotTask {
    async fn batch_run(_: Vec<Self>) {
        flush_tree_snapshot_task();
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
            let encode_start = Instant::now();
            let (snapshot_bytes, layout) = match entry_ref.encode_with_layout() {
                Ok(encoded) => encoded,
                Err(e) => {
                    handle_error(anyhow::anyhow!(
                        "FlushTreeSnapshotTask: Failed to encode snapshot {timestamp}: {e}"
                    ));
                    break;
                }
            };
            crate::perf_timing!(
                "tree_snapshot.encode_compact",
                encode_start,
                "Encode compact ordinal snapshot"
            );
            let scrollbar_bytes = match codec::encode(&entry_ref.scrollbar) {
                Ok(bytes) => bytes,
                Err(e) => {
                    handle_error(anyhow::anyhow!(
                        "FlushTreeSnapshotTask: Failed to encode scrollbar for timestamp {timestamp}: {e}"
                    ));
                    break;
                }
            };

            let timer_start = Instant::now();
            let txn = match TREE_SNAPSHOT.in_disk.begin_write() {
                Ok(t) => t,
                Err(e) => {
                    handle_error(anyhow::anyhow!(
                        "FlushTreeSnapshotTask: Failed to begin write transaction: {e}"
                    ));
                    break;
                }
            };
            {
                let mut metadata = match txn.open_table(SCROLLBAR_METADATA_TABLE) {
                    Ok(t) => t,
                    Err(e) => {
                        handle_error(anyhow::anyhow!(
                            "FlushTreeSnapshotTask: Failed to open scrollbar metadata table: {e}"
                        ));
                        break;
                    }
                };
                if let Err(e) = metadata.insert(timestamp, scrollbar_bytes.as_slice()) {
                    handle_error(anyhow::anyhow!(
                        "FlushTreeSnapshotTask: Failed to insert scrollbar metadata for timestamp {timestamp}: {e}"
                    ));
                    break;
                }
            }

            {
                let mut table = match txn.open_table(TREE_SNAPSHOT_TABLE) {
                    Ok(t) => t,
                    Err(e) => {
                        handle_error(anyhow::anyhow!(
                            "FlushTreeSnapshotTask: Failed to open snapshot table: {e}"
                        ));
                        break;
                    }
                };
                if let Err(e) = table.insert(timestamp, snapshot_bytes.as_slice()) {
                    handle_error(anyhow::anyhow!(
                        "FlushTreeSnapshotTask: Failed to insert snapshot {timestamp}: {e}"
                    ));
                    break;
                }
            }

            if let Err(e) = txn.commit() {
                handle_error(anyhow::anyhow!(
                    "FlushTreeSnapshotTask: Failed to commit transaction for timestamp {timestamp}: {e}"
                ));
                break;
            }
            TREE_SNAPSHOT.verified_layouts.insert(timestamp, layout);

            crate::perf_timing!(
                "tree_snapshot.flush_disk",
                timer_start,
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
}
