use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use anyhow::Result;
use mini_executor::Task;
use redb::TableDefinition;
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
    let write_txn = TREE_SNAPSHOT.in_disk.begin_write().unwrap();
    let binding = timestamp.to_string();
    let table_definition: TableDefinition<u64, u64> = TableDefinition::new(&binding);

    match write_txn.delete_table(table_definition) {
        Ok(true) => {
            info!("Delete tree cache table: {:?}", timestamp);
        }
        Ok(false) => {
            error!("Failed to delete tree cache table: {:?}", timestamp);
        }
        Err(err) => {
            error!(
                "Failed to delete tree cache table: {:?}, error: {:#?}",
                timestamp, err
            );
        }
    }

    info!(
        "{} items remaining in disk tree cache",
        write_txn.list_tables().unwrap().count()
    );

    write_txn.commit().unwrap();
}
