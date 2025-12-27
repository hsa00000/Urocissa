use crate::operations::utils::timestamp::get_current_timestamp_u64;
use crate::public::db::expire::{EXPIRE, EXPIRE_TABLE_DEFINITION};
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::expire_check::ExpireCheckTask;
use mini_executor::BatchTask;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub struct UpdateExpireTask;

impl BatchTask for UpdateExpireTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            update_expire_task();
        }
    }
}

fn update_expire_task() {
    let current_timestamp = get_current_timestamp_u64();
    let last_timestamp = VERSION_COUNT_TIMESTAMP.swap(current_timestamp, Ordering::SeqCst);

    if last_timestamp > 0 {
        let expire_write_txn = EXPIRE.in_disk.begin_write().unwrap();
        let new_expire_time =
            current_timestamp.saturating_add(Duration::from_secs(60 * 60).as_millis() as u64);
        {
            let mut expire_table = expire_write_txn
                .open_table(EXPIRE_TABLE_DEFINITION)
                .expect("Failed to open expire table");

            expire_table
                .insert(last_timestamp, Some(new_expire_time))
                .expect("Failed to insert into expire table");
            expire_table
                .insert(current_timestamp, None)
                .expect("Failed to insert into expire table");

            info!(
                "Expire table updated. Next expire time set to {}",
                new_expire_time
            );
        }

        expire_write_txn.commit().unwrap();
        BATCH_COORDINATOR.execute_batch_detached(ExpireCheckTask);
    }
}
