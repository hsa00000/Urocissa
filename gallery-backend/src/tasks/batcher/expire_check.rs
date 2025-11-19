use crate::public::constant::SNAPSHOT_MAX_LIFETIME_MS;
use crate::public::db::sqlite::SQLITE;
use crate::tasks::looper::reset_expire_check_timer;
use log::error;
use mini_executor::BatchTask;
use std::time::{SystemTime, UNIX_EPOCH};

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
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let threshold = now - SNAPSHOT_MAX_LIFETIME_MS as u128;
    if let Err(e) = SQLITE.delete_expired_snapshots(threshold) {
        error!("Failed to delete expired snapshots: {}", e);
    }
    if let Err(e) = SQLITE.delete_expired_pending_data(threshold) {
        error!("Failed to delete expired pending data: {}", e);
    }
}

