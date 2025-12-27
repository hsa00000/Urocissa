use crate::public::constant::MAX_DELETE_ATTEMPTS;
use crate::public::error_data::handle_error;
use anyhow::Context;
use anyhow::Result;
use mini_executor::Task;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
    thread,
    time::Duration,
};
use tokio::task::spawn_blocking;

static UPLOAD_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| fs::canonicalize("./upload").expect("`./upload` directory must exist"));

pub struct DeleteTask {
    pub path: PathBuf,
}

impl DeleteTask {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Task for DeleteTask {
    type Output = Result<()>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            spawn_blocking(move || delete_in_upload_task(self.path))
                .await
                .expect("blocking task panicked")
                .map_err(|err| handle_error(err.context("Failed to run delete task")))
        }
    }
}
fn delete_in_upload_task(path: PathBuf) -> Result<()> {
    // Skip if path is not under ./upload
    if !path_starts_with_upload(&path) {
        return Ok(());
    }

    let mut attempts = 0;
    loop {
        attempts += 1;
        match fs::remove_file(&path) {
            Ok(_) => {
                log::info!("Deleted file: {:?}", path);
                return Ok(());
            }
            Err(err) if attempts < MAX_DELETE_ATTEMPTS => {
                log::warn!(
                    "Failed deleting {:?} (attempt {}), retrying in {}ms: {}",
                    path,
                    attempts,
                    100 * attempts,
                    err
                );
                thread::sleep(Duration::from_millis(100 * attempts));
            }
            Err(err) => {
                return Err(err).context(format!(
                    "Failed deleting {:?} after {} attempts",
                    path, attempts
                ));
            }
        }
    }
}

pub fn path_starts_with_upload(path: &Path) -> bool {
    match fs::canonicalize(path) {
        Ok(abs_path) => abs_path.starts_with(&*UPLOAD_PATH),
        Err(_) => false,
    }
}
