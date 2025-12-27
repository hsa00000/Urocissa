use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::response::reduced_data::ReducedData;
use mini_executor::BatchTask;
use redb::TableDefinition;
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
            let timestamp_str = timestamp.to_string();

            let timer_start = Instant::now();
            let txn = match TREE_SNAPSHOT.in_disk.begin_write() {
                Ok(t) => t,
                Err(e) => {
                    handle_error(anyhow::anyhow!(
                        "FlushTreeSnapshotTask: Failed to begin write transaction: {}",
                        e
                    ));
                    break;
                }
            };
            let table_definition: TableDefinition<u64, ReducedData> =
                TableDefinition::new(&timestamp_str);

            {
                let mut table = match txn.open_table(table_definition) {
                    Ok(t) => t,
                    Err(e) => {
                        handle_error(anyhow::anyhow!(
                            "FlushTreeSnapshotTask: Failed to open table {}: {}",
                            timestamp_str,
                            e
                        ));
                        break;
                    }
                };
                for (index, data) in entry_ref.iter().enumerate() {
                    if let Err(e) = table.insert(index as u64, data) {
                        handle_error(anyhow::anyhow!(
                            "FlushTreeSnapshotTask: Failed to insert data at index {} for timestamp {}: {}",
                            index,
                            timestamp,
                            e
                        ));
                    }
                }
            }

            if let Err(e) = txn.commit() {
                handle_error(anyhow::anyhow!(
                    "FlushTreeSnapshotTask: Failed to commit transaction for timestamp {}: {}",
                    timestamp,
                    e
                ));
                break;
            }

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
}
