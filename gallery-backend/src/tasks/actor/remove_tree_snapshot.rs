use anyhow::Result;
use mini_executor::Task;
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
        }
    }
}

fn remove_task(_timestamp: u128) -> Result<()> {
    // TODO: Implement SQLite snapshot removal if needed
    Ok(())
}

