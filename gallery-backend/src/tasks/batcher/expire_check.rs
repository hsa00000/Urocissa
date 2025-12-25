use crate::public::db::query_snapshot::QUERY_SNAPSHOT;
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::router::get::get_prefetch::Prefetch;
use crate::{public::db::expire::EXPIRE, tasks::INDEX_COORDINATOR};

use crate::tasks::actor::remove_tree_snapshot::RemoveTask;
use crate::tasks::looper::reset_expire_check_timer;
use mini_executor::BatchTask;
use rayon::iter::{ParallelBridge, ParallelIterator};

use log::{error, info};
use redb::{ReadableDatabase, ReadableTable, TableDefinition, TableHandle};
use std::sync::atomic::Ordering;

pub struct ExpireCheckTask;

impl BatchTask for ExpireCheckTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            expire_check_task();
            // Reset countdown timer after task execution
            reset_expire_check_timer().await;
        }
    }
}

fn expire_check_task() {
    let write_txn = QUERY_SNAPSHOT.in_disk.begin_write().unwrap();

    write_txn
        .list_tables()
        .unwrap()
        .par_bridge()
        .for_each(|table_handle| {
            if let Ok(timestamp) = table_handle.name().parse::<u64>()
                && VERSION_COUNT_TIMESTAMP.load(Ordering::Relaxed) > timestamp
                && EXPIRE.expired_check(timestamp)
            {
                let binding = timestamp.to_string();
                let table_definition: TableDefinition<u64, Prefetch> =
                    TableDefinition::new(&binding);

                let read_txn = QUERY_SNAPSHOT.in_disk.begin_read().unwrap();
                let table = read_txn.open_table(table_definition).unwrap();

                match write_txn.delete_table(table_handle) {
                    Ok(true) => {
                        info!("Delete query cache table: {:?}", timestamp);
                        let tree_snapshot_delete_queue: Vec<_> = table
                            .iter()
                            .unwrap()
                            .par_bridge()
                            .map(|result| {
                                let (_, guard) = result.unwrap();
                                let prefetch_return = guard.value();
                                prefetch_return.timestamp
                            })
                            .collect();

                        for timestamp in tree_snapshot_delete_queue {
                            let _ = INDEX_COORDINATOR.execute_detached(RemoveTask::new(timestamp));
                        }
                    }
                    Ok(false) => {
                        error!("Failed to delete query cache table: {:?}", timestamp);
                    }
                    Err(err) => {
                        error!(
                            "Failed to delete query cache table: {:?}, error: {:#?}",
                            timestamp, err
                        );
                    }
                }

                info!(
                    "{} items remaining in disk query cache",
                    write_txn.list_tables().unwrap().count()
                );
            }
        });

    write_txn.commit().unwrap();
}
