use crate::public::db::query_snapshot::QUERY_SNAPSHOT;
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::router::get::get_prefetch::Prefetch;

use mini_executor::BatchTask;
use redb::TableDefinition;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::public::error_data::handle_error;
use anyhow;

pub struct FlushQuerySnapshotTask;

impl BatchTask for FlushQuerySnapshotTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            flush_query_snapshot_task();
        }
    }
}

fn flush_query_snapshot_task() {
    loop {
        if QUERY_SNAPSHOT.in_memory.is_empty() {
            break;
        }

        // Narrow scope for the DashMap reference
        let expression_hashed = {
            // Attempt to get a reference to one entry:
            let Some(entry_ref) = QUERY_SNAPSHOT.in_memory.iter().next() else {
                break;
            };

            let expression_hashed = *entry_ref.key();
            let ref_data = entry_ref.value();

            // Save to disk
            let timer_start = Instant::now();
            let txn = match QUERY_SNAPSHOT.in_disk.begin_write() {
                Ok(t) => t,
                Err(e) => {
                    handle_error(anyhow::anyhow!(
                        "FlushQuerySnapshotTask: Failed to begin write transaction: {}",
                        e
                    ));
                    break;
                }
            };
            let count_version = &VERSION_COUNT_TIMESTAMP.load(Ordering::Relaxed).to_string();
            let table_definition: TableDefinition<u64, Prefetch> =
                TableDefinition::new(count_version);

            {
                let mut table = match txn.open_table(table_definition) {
                    Ok(t) => t,
                    Err(e) => {
                        handle_error(anyhow::anyhow!(
                            "FlushQuerySnapshotTask: Failed to open table {}: {}",
                            count_version,
                            e
                        ));
                        break;
                    }
                };
                if let Err(e) = table.insert(expression_hashed, ref_data) {
                    handle_error(anyhow::anyhow!(
                        "FlushQuerySnapshotTask: Failed to insert data for expression_hashed {}: {}",
                        expression_hashed,
                        e
                    ));
                }
            }

            if let Err(e) = txn.commit() {
                handle_error(anyhow::anyhow!(
                    "FlushQuerySnapshotTask: Failed to commit transaction for expression_hashed {}: {}",
                    expression_hashed,
                    e
                ));
                break;
            }
            info!(
                duration = &*format!("{:?}", timer_start.elapsed());
                "Write query cache into disk"
            );

            // Return the hashed key, so we can remove it below
            expression_hashed
        };

        // Remove from DashMap *after* reference is dropped
        QUERY_SNAPSHOT.in_memory.remove(&expression_hashed);

        info!(
            "{} items remaining in in-memory query cache",
            QUERY_SNAPSHOT.in_memory.len()
        );
    }
}
