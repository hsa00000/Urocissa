use super::QuerySnapshot;
use crate::public::{
    query_snapshot::PrefetchReturn, tree::start_loop::VERSION_COUNT,
    tree_snapshot::start_loop::TREE_SNAPSHOT_DELETE_QUEUE_SENDER, utils::expired,
};
use rayon::iter::{ParallelBridge, ParallelIterator};
use redb::{ReadableTable, TableDefinition, TableHandle};
use std::{
    sync::atomic::Ordering,
    thread::sleep,
    time::{Duration, Instant},
};
use tokio::sync::Notify;

pub static SHOULD_FLUSH_QUERY_SNAPSHOT: Notify = Notify::const_new();

impl QuerySnapshot {
    pub fn start_loop(&self) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn_blocking(|| loop {
            sleep(Duration::from_millis(500));

            let write_txn = self.in_disk.begin_write().unwrap();

            /*  let list: Vec<String> = write_txn
                .list_tables()
                .unwrap()
                .map(|x| x.name().to_string())
                .collect();
            println!("list is {:?}", list); */

            write_txn
                .list_tables()
                .unwrap()
                .par_bridge()
                .for_each(|table_handle| {
                    let timestamp = table_handle.name().parse::<u64>().unwrap();
                    println!("VERSION_COUNT.load(Ordering::Relaxed) is {}", VERSION_COUNT.load(Ordering::Relaxed));
                    println!("expired(timestamp) is {}", expired(timestamp));
                    println!("VERSION_COUNT.load(Ordering::Relaxed) > timestamp && expired(timestamp) is {}", VERSION_COUNT.load(Ordering::Relaxed) > timestamp && expired(timestamp));
                    if VERSION_COUNT.load(Ordering::Relaxed) > timestamp && expired(timestamp) {
                        // 1 hours in milliseconds
                        let binding = timestamp.to_string();
                        let table_definition: TableDefinition<u64, PrefetchReturn> =
                            TableDefinition::new(&binding);
                        let read_txn = self.in_disk.begin_read().unwrap();
                        let table = read_txn.open_table(table_definition).unwrap();
                        let tree_snapshot_delete_queue: Vec<_> = table
                            .iter()
                            .unwrap()
                            .filter_map(|result| {
                                let (_, value) = result.unwrap();
                                let prefetch_return = value.value();
                                prefetch_return.map(|prefetch| prefetch.timestamp)
                            })
                            .collect();

                        match write_txn.delete_table(table_handle) {
                            Ok(true) => {
                                info!("Delete table: {:?}", timestamp);
                                TREE_SNAPSHOT_DELETE_QUEUE_SENDER
                                    .get()
                                    .unwrap()
                                    .send(tree_snapshot_delete_queue)
                                    .unwrap();
                            }
                            Ok(false) => error!("Failed to delete table: {:?}", timestamp),
                            Err(e) => {
                                error!("Failed to delete table: {:?}, error: {:?}", timestamp, e)
                            }
                        }
                    }
                });
            write_txn.commit().unwrap();
        });
        tokio::task::spawn(async {
            loop {
                SHOULD_FLUSH_QUERY_SNAPSHOT.notified().await;
                tokio::task::spawn_blocking(|| loop {
                    if self.in_memory.len() > 0 {
                        let mut expression_hashed_opt = None;
                        let mut ref_data_opt = None;
                        {
                            if let Some(ref_data) = self.in_memory.iter().next() {
                                expression_hashed_opt = Some(ref_data.key().clone());
                                ref_data_opt = Some(ref_data.value().clone());
                            }
                        }

                        if let Some(expression_hashed) = expression_hashed_opt {
                            if let Some(ref_data) = ref_data_opt {
                                {
                                    let timer_start = Instant::now();
                                    let txn = self.in_disk.begin_write().unwrap();
                                    let count_version =
                                        &VERSION_COUNT.load(Ordering::Relaxed).to_string();
                                    let table_definition: TableDefinition<u64, PrefetchReturn> =
                                        TableDefinition::new(&count_version);
                                    {
                                        let mut table = txn.open_table(table_definition).unwrap();

                                        table.insert(expression_hashed, ref_data).unwrap();
                                    }
                                    txn.commit().unwrap();
                                    info!(duration = &*format!("{:?}", timer_start.elapsed());
                                        "Write query cache into disk"
                                    );
                                }
                                {
                                    self.in_memory.remove(&expression_hashed);
                                    info!(
                                        "{} items remaining in in-memory query cache",
                                        self.in_memory.len()
                                    );
                                }
                            }
                        }
                    } else {
                        break;
                    }
                })
                .await
                .unwrap();
            }
        })
    }
}
