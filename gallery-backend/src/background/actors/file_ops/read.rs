use crate::common::errors::handle_error;
use anyhow::{Error, Result};
use log::warn;
use mini_executor::Task;
use std::{
    fs::File,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};
use tokio::task::spawn_blocking;

const OPEN_FAIL_RETRY: usize = 3;
const OPEN_RETRY_DELAY_MS: u64 = 100;

pub struct OpenFileTask {
    pub path: PathBuf,
}

impl OpenFileTask {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl Task for OpenFileTask {
    type Output = Result<File>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || open_file_with_retry(self.path))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run hash task")))
        }
    }
}

pub fn open_file_with_retry<P: AsRef<Path> + std::fmt::Debug>(path: P) -> Result<File> {
    let mut delay = Duration::from_millis(OPEN_RETRY_DELAY_MS);

    for attempt in 0..=OPEN_FAIL_RETRY {
        match File::open(&path) {
            Ok(file) => return Ok(file),
            Err(e) if attempt < OPEN_FAIL_RETRY => {
                warn!(
                    "Attempt {}/{} failed to open {:?}: {}. Retrying in {:?}…",
                    attempt + 1,
                    OPEN_FAIL_RETRY + 1,
                    path,
                    e,
                    delay,
                );
                sleep(delay);
                delay = delay.checked_mul(2).unwrap_or(delay);
            }
            Err(e) => {
                return Err(Error::new(e).context(format!(
                    "Failed to open file {:?} after {} attempts",
                    path,
                    OPEN_FAIL_RETRY + 1
                )));
            }
        }
    }

    unreachable!("open_file_with_retry logic error")
}
