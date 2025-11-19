use crate::tasks::looper::reset_expire_check_timer;
use mini_executor::BatchTask;

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
    // TODO: Implement SQLite expiration logic if needed
}

