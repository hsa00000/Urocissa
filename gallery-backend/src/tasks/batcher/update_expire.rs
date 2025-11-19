use crate::operations::utils::timestamp::get_current_timestamp_u64;
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::expire_check::ExpireCheckTask;
use mini_executor::BatchTask;
use std::sync::atomic::Ordering;

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
        // Trigger the check task to clean up old snapshots
        BATCH_COORDINATOR.execute_batch_detached(ExpireCheckTask);
    }
}

