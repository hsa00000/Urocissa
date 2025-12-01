use crate::public::{
    db::tree_snapshot::TREE_SNAPSHOT, error_data::handle_error,
    structure::reduced_data::ReducedData,
};
use anyhow::Result;
use mini_executor::Task;
use redb::TableDefinition;
use tokio::task::spawn_blocking;
pub struct RemoveTask {
    pub timestamp: u128,
}

impl RemoveTask {
    pub fn new(timestamp: u128) -> Self {
        Self { timestamp }
    }
}

impl Task for RemoveTask {
    type Output = Result<()>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || remove_task(self.timestamp))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run remove task")))
        }
    }
}
/// Removes a tree cache table by its timestamp.
fn remove_task(timestamp: u128) -> Result<()> {
    let write_txn = TREE_SNAPSHOT.in_disk.begin_write().unwrap();
    let binding = timestamp.to_string();
    let table_definition: TableDefinition<u64, ReducedData> = TableDefinition::new(&binding);

    match write_txn.delete_table(table_definition) {
        Ok(true) => {
            info!("Delete tree cache table: {:?}", timestamp)
        }
        Ok(false) => {
            error!("Failed to delete tree cache table: {:?}", timestamp)
        }
        Err(err) => {
            error!(
                "Failed to delete tree cache table: {:?}, error: {:#?}",
                timestamp, err
            )
        }
    }

    info!(
        "{} items remaining in disk tree cache",
        write_txn.list_tables().unwrap().count()
    );

    write_txn.commit().unwrap();
    Ok(())
}
