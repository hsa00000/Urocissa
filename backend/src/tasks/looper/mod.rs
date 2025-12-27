use crate::public::constant::{SNAPSHOT_MAX_LIFETIME_MS, runtime::INDEX_RUNTIME};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::expire_check::ExpireCheckTask;
use std::sync::{Arc, LazyLock};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

static RESET_SENDER: LazyLock<Arc<tokio::sync::Mutex<Option<mpsc::UnboundedSender<()>>>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(None)));

pub fn start_expire_check_loop() {
    INDEX_RUNTIME.spawn(async {
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Store the sender for external reset usage
        {
            let mut sender = RESET_SENDER.lock().await;
            *sender = Some(tx);
        }

        loop {
            // Wait for SNAPSHOT_MAX_LIFETIME_MS or receive a reset signal
            let sleep_future = sleep(Duration::from_millis(SNAPSHOT_MAX_LIFETIME_MS));

            tokio::select! {
                _ = sleep_future => {
                    // Timeout reached, execute the check task
                    info!("Timeout reached, executing the check task");
                    BATCH_COORDINATOR.execute_batch_detached(ExpireCheckTask);
                }
                _ = rx.recv() => {
                    // Received reset signal, restart the timer
                    continue;
                }
            }
        }
    });
}

// Provide a function to reset the countdown timer
pub async fn reset_expire_check_timer() {
    if let Some(sender) = RESET_SENDER.lock().await.as_ref() {
        let _ = sender.send(());
    }
}
