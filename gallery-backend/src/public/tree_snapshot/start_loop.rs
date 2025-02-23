use super::TreeSnapshot;
use crate::public::{reduced_data::ReducedData, utils::start_loop_util};
use redb::TableDefinition;
use std::{
    collections::HashSet,
    sync::{Arc, OnceLock},
    time::Instant,
};
use tokio::sync::{Notify, mpsc::UnboundedSender};

static TREE_SNAPSHOT_FLUSH_SENDER: OnceLock<UnboundedSender<Option<Arc<Notify>>>> = OnceLock::new();

static TREE_SNAPSHOT_DELETE_SENDER: OnceLock<UnboundedSender<TreeSnapshotDelete>> = OnceLock::new();

pub struct TreeSnapshotDelete {
    pub timestamp_list: Vec<u128>,
    pub notify: Option<Arc<Notify>>,
}

impl TreeSnapshot {
    // Delete snapshots send from EXPIRE.
    pub fn start_loop_remove(&'static self) -> tokio::task::JoinHandle<()> {
        start_loop_util(&TREE_SNAPSHOT_DELETE_SENDER, |buffer| {
            let unique_timestamp: HashSet<_> = buffer
                .iter()
                .flat_map(|tree_snapshot_delete| tree_snapshot_delete.timestamp_list.iter()) // Flatten all album_list vectors
                .collect();
            let tree_snapshot_delete_queue: Vec<_> = unique_timestamp.into_iter().collect();
            tree_snapshot_delete_queue
                .iter()
                .for_each(|timestamp_delete| {
                    let write_txn = self.in_disk.begin_write().unwrap();
                    let binding = timestamp_delete.to_string();
                    let table_definition: TableDefinition<u64, ReducedData> =
                        TableDefinition::new(&binding);

                    match write_txn.delete_table(table_definition) {
                        Ok(true) => {
                            info!("Delete tree cache table: {:?}", timestamp_delete)
                        }
                        Ok(false) => {
                            error!("Failed to delete tree cache table: {:?}", timestamp_delete)
                        }
                        Err(e) => {
                            error!(
                                "Failed to delete tree cache table: {:?}, error: {:#?}",
                                timestamp_delete, e
                            )
                        }
                    }

                    info!(
                        "{} items remaining in disk tree cache",
                        write_txn.list_tables().unwrap().count()
                    );

                    write_txn.commit().unwrap();
                    buffer.iter().for_each(|tree_snapshot_delete| {
                        if let Some(notify) = &tree_snapshot_delete.notify {
                            notify.notify_one()
                        };
                    });
                });
        })
    }

    pub fn tree_snapshot_delete(&self, timestamp_list: Vec<u128>) {
        TREE_SNAPSHOT_DELETE_SENDER
            .get()
            .unwrap()
            .send(TreeSnapshotDelete {
                timestamp_list: timestamp_list,
                notify: None,
            })
            .unwrap();
    }
    pub async fn _tree_snapshot_delete_async(&self, timestamp_list: Vec<u128>) {
        let notify = Arc::new(Notify::new());
        TREE_SNAPSHOT_DELETE_SENDER
            .get()
            .unwrap()
            .send(TreeSnapshotDelete {
                timestamp_list: timestamp_list,
                notify: Some(notify.clone()),
            })
            .unwrap();
        notify.notified().await
    }

    // Flush snapshots in memory to disk
    pub fn start_loop_flush(&'static self) -> tokio::task::JoinHandle<()> {
        start_loop_util(&TREE_SNAPSHOT_FLUSH_SENDER, |buffer| {
            loop {
                if self.in_memory.is_empty() {
                    break;
                }

                // Narrow scope for the DashMap reference
                let timestamp = {
                    // Attempt to get a reference to one entry:
                    let Some(entry_ref) = self.in_memory.iter().next() else {
                        break;
                    };

                    let timestamp = *entry_ref.key();
                    let timestamp_str = timestamp.to_string();

                    let timer_start = Instant::now();
                    let txn = self.in_disk.begin_write().unwrap();
                    let table_definition: TableDefinition<u64, ReducedData> =
                        TableDefinition::new(&timestamp_str);

                    {
                        let mut table = txn.open_table(table_definition).unwrap();
                        for (index, data) in entry_ref.iter().enumerate() {
                            table.insert(index as u64, data).unwrap();
                        }
                    }

                    txn.commit().unwrap();

                    info!(
                        duration = &*format!("{:?}", timer_start.elapsed());
                        "Write in-memory cache into disk"
                    );
                    timestamp
                };

                //Remove from DashMap *after* reference is dropped
                self.in_memory.remove(&timestamp);
                info!(
                    "{} items remaining in in-memory tree cache",
                    self.in_memory.len()
                );
                buffer.iter().for_each(|notify_opt| {
                    if let Some(notify) = notify_opt {
                        notify.notify_one()
                    }
                });
            }
        })
    }
    pub fn tree_snapshot_flush(&self) {
        TREE_SNAPSHOT_FLUSH_SENDER
            .get()
            .unwrap()
            .send(None)
            .unwrap();
    }
    pub async fn _tree_snapshot_flush_async(&self) {
        let notify = Arc::new(Notify::new());
        TREE_SNAPSHOT_FLUSH_SENDER
            .get()
            .unwrap()
            .send(Some(notify.clone()))
            .unwrap();
        notify.notified().await
    }
}
