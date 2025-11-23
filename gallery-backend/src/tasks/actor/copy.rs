use anyhow::Context;
use anyhow::Result;
use mini_executor::Task;
use std::fs;
use std::sync::LazyLock;
use tokio::sync::Semaphore;
use tokio::task::spawn_blocking;

use crate::process::io::copy_with_retry;
use crate::public::error_data::handle_error;
use crate::public::structure::database_struct::database::definition::Database;

static COPY_LIMIT: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::const_new(1));

pub struct CopyTask {
    pub database: Database,
}

impl CopyTask {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

impl Task for CopyTask {
    type Output = Result<Database>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _permit = COPY_LIMIT.acquire().await?;
            spawn_blocking(move || copy_task(self.database))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run copy task")))
        }
    }
}

fn copy_task(database: Database) -> Result<Database> {
    let source_path = database.source_path();
    let dest_path = database.imported_path();

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory tree for {:?}", parent))?;
    }

    copy_with_retry(&source_path, &dest_path).with_context(|| {
        format!(
            "failed to copy file from {:?} to {:?}",
            source_path, dest_path
        )
    })?; // If it fails three times, it goes into the Err branch

    Ok(database)
}
