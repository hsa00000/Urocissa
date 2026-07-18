use crate::public::db::tree_snapshot::{
    SCROLLBAR_METADATA_TABLE, TREE_SNAPSHOT, TREE_SNAPSHOT_TABLE,
};
use anyhow::Result;
use mini_executor::Task;
use redb::ReadableTableMetadata;
use tokio::task::spawn_blocking;
pub struct RemoveTask {
    pub timestamp: i64,
}

impl RemoveTask {
    pub fn new(timestamp: i64) -> Self {
        Self { timestamp }
    }
}

impl Task for RemoveTask {
    type Output = Result<()>;

    async fn run(self) -> Self::Output {
        spawn_blocking(move || remove_task(self.timestamp))
            .await
            .expect("blocking task panicked");
        Ok(())
    }
}
/// Removes a tree cache table by its timestamp.
fn remove_task(timestamp: i64) {
    TREE_SNAPSHOT.in_memory.remove(&timestamp);
    TREE_SNAPSHOT.verified_layouts.remove(&timestamp);
    let write_txn = TREE_SNAPSHOT.in_disk.begin_write().unwrap();
    match write_txn.open_table(TREE_SNAPSHOT_TABLE) {
        Ok(mut table) => match table.remove(timestamp) {
            Ok(Some(_)) => info!("Delete tree cache snapshot: {timestamp}"),
            Ok(None) => error!("Tree cache snapshot did not exist: {timestamp}"),
            Err(err) => error!("Failed to delete tree cache snapshot {timestamp}: {err:#?}"),
        },
        Err(err) => error!("Failed to open tree cache snapshot table: {err:#?}"),
    }

    match write_txn.open_table(SCROLLBAR_METADATA_TABLE) {
        Ok(mut table) => {
            if let Err(err) = table.remove(timestamp) {
                error!("Failed to delete scrollbar metadata for {timestamp}: {err:#?}");
            }
        }
        Err(err) => {
            error!("Failed to open scrollbar metadata table: {err:#?}");
        }
    }

    let remaining = match write_txn.open_table(TREE_SNAPSHOT_TABLE) {
        Ok(table) => table.len().unwrap_or(0),
        Err(_) => 0,
    };
    info!("{remaining} items remaining in disk tree cache");

    write_txn.commit().unwrap();
}
