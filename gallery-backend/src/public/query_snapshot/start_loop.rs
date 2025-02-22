use super::QuerySnapshot;
use crate::public::{
    query_snapshot::Prefetch, tree::start_loop::VERSION_COUNT_TIMESTAMP, utils::start_loop_util,
};
use redb::TableDefinition;
use std::{
    sync::{Arc, OnceLock, atomic::Ordering},
    time::Instant,
};
use tokio::sync::{Notify, mpsc::UnboundedSender};

static QUERY_SNAPSHOT_FLUSH_SENDER: OnceLock<UnboundedSender<Option<Arc<Notify>>>> =
    OnceLock::new();

impl QuerySnapshot {
    pub fn start_loop(&'static self) -> tokio::task::JoinHandle<()> {
        start_loop_util(&QUERY_SNAPSHOT_FLUSH_SENDER, |buffer| {
            loop {
                if self.in_memory.is_empty() {
                    break;
                }

                // Narrow scope for the DashMap reference
                let expression_hashed = {
                    // Attempt to get a reference to one entry:
                    let Some(entry_ref) = self.in_memory.iter().next() else {
                        break;
                    };

                    let expression_hashed = *entry_ref.key();
                    let ref_data = entry_ref.value();

                    // Save to disk
                    let timer_start = Instant::now();
                    let txn = self.in_disk.begin_write().unwrap();
                    let count_version =
                        &VERSION_COUNT_TIMESTAMP.load(Ordering::Relaxed).to_string();
                    let table_definition: TableDefinition<u64, Prefetch> =
                        TableDefinition::new(count_version);

                    {
                        let mut table = txn.open_table(table_definition).unwrap();
                        table.insert(expression_hashed, ref_data).unwrap();
                    }

                    txn.commit().unwrap();
                    info!(
                        duration = &*format!("{:?}", timer_start.elapsed());
                        "Write query cache into disk"
                    );

                    // Return the hashed key, so we can remove it below
                    expression_hashed
                };

                // Remove from DashMap *after* reference is dropped
                self.in_memory.remove(&expression_hashed);

                info!(
                    "{} items remaining in in-memory query cache",
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
    pub fn query_snapshot_flush(&self) {
        QUERY_SNAPSHOT_FLUSH_SENDER
            .get()
            .unwrap()
            .send(None)
            .unwrap();
    }
    pub async fn _query_snapshot_flus_async(&self) {
        let notify = Arc::new(Notify::new());
        QUERY_SNAPSHOT_FLUSH_SENDER
            .get()
            .unwrap()
            .send(Some(notify.clone()))
            .unwrap();
        notify.notified().await
    }
}
