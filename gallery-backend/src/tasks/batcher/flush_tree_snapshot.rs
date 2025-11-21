use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::reduced_data::ReducedData;
use anyhow::Result;
use log::error;
use mini_executor::BatchTask;
use redb::TableDefinition;
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
            let timestamp_str = timestamp.to_string();

            let timer_start = Instant::now();
            let txn = TREE_SNAPSHOT.in_disk.begin_write()?;
            let table_definition: TableDefinition<u64, ReducedData> =
                TableDefinition::new(&timestamp_str);

            {
                let mut table = txn.open_table(table_definition)?;
                for (index, data) in entry_ref.iter().enumerate() {
                    table.insert(index as u64, data)?;
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
