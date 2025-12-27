use crate::{operations::open_file::open_file_with_retry, public::error_data::handle_error};
use anyhow::Result;
use mini_executor::Task;
use std::{fs::File, path::PathBuf};
use tokio::task::spawn_blocking;

pub struct OpenFileTask {
    pub path: PathBuf,
}

impl OpenFileTask {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Task for OpenFileTask {
    type Output = Result<File>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || open_file_task(self.path))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run hash task")))
        }
    }
}
fn open_file_task(path: PathBuf) -> Result<File> {
    open_file_with_retry(path)
}
