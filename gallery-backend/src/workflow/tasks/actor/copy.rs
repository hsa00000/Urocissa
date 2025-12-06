use anyhow::Context;
use anyhow::Result;
use mini_executor::Task;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::sync::Semaphore;
use tokio::task::spawn_blocking;

use crate::public::error_data::handle_error;
use crate::public::io::copy_with_retry;
use crate::public::structure::abstract_data::AbstractData;
use crate::utils::imported_path;

static COPY_LIMIT: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::const_new(1));

pub struct CopyTask {
    pub path: PathBuf,
    pub data: AbstractData,
}

impl CopyTask {
    pub fn new(path: impl Into<PathBuf>, data: AbstractData) -> Self {
        Self {
            path: path.into(),
            data,
        }
    }
}

impl Task for CopyTask {
    type Output = Result<AbstractData>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            let _permit = COPY_LIMIT.acquire().await?;
            spawn_blocking(move || copy_task(self))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run copy task")))
        }
    }
}

fn copy_task(task: CopyTask) -> Result<AbstractData> {
    let source_path = task.path;
    let dest_path = match &task.data {
        AbstractData::Image(i) => imported_path(i.object.id, &i.metadata.ext),
        AbstractData::Video(v) => imported_path(v.object.id, &v.metadata.ext),
        _ => return Err(anyhow::anyhow!("Unsupported type")),
    };

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

    Ok(task.data)
}
