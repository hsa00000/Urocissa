use crate::tasks::{
    INDEX_COORDINATOR,
    actor::{
        copy::CopyTask, deduplicate::DeduplicateTask, delete_in_update::DeleteTask, hash::HashTask,
        index::IndexTask, open_file::OpenFileTask, video::VideoTask,
    },
};
use anyhow::Result;
use arrayvec::ArrayString;
use dashmap::DashSet;
use log::warn;
use path_clean::PathClean;
use std::{path::PathBuf, sync::LazyLock};

static IN_PROGRESS: LazyLock<DashSet<ArrayString<64>>> = LazyLock::new(DashSet::new);

pub struct ProcessingGuard(ArrayString<64>);
impl Drop for ProcessingGuard {
    fn drop(&mut self) {
        IN_PROGRESS.remove(&self.0);
    }
}

fn try_acquire(hash: ArrayString<64>) -> Option<ProcessingGuard> {
    if IN_PROGRESS.insert(hash.clone()) {
        Some(ProcessingGuard(hash))
    } else {
        None
    }
}

pub async fn index_for_watch(
    path: PathBuf,
    presigned_album_id_opt: Option<ArrayString<64>>,
) -> Result<()> {
    let path = path.clean();
    let file = INDEX_COORDINATOR
        .execute_waiting(OpenFileTask::new(path.clone()))
        .await??;

    let hash = INDEX_COORDINATOR
        .execute_waiting(HashTask::new(file))
        .await??;

    let _guard = match try_acquire(hash) {
        Some(g) => g,
        None => {
            warn!(
                "Processing already in progress for path: {:?}, hash: {}",
                path, hash
            );
            return Ok(());
        }
    };

    let abstract_data_opt = INDEX_COORDINATOR
        .execute_waiting(DeduplicateTask::new(
            path.clone(),
            hash,
            presigned_album_id_opt,
        ))
        .await??;

    // If the file is already in the database, we can skip further processing.
    let mut abstract_data = match abstract_data_opt {
        Some(data) => data,
        None => {
            INDEX_COORDINATOR.execute_detached(DeleteTask::new(path));
            return Ok(());
        }
    };

    abstract_data = INDEX_COORDINATOR
        .execute_waiting(CopyTask::new(abstract_data))
        .await??;
    abstract_data = INDEX_COORDINATOR
        .execute_waiting(IndexTask::new(abstract_data))
        .await??;

    INDEX_COORDINATOR.execute_detached(DeleteTask::new(PathBuf::from(&path)));
    if abstract_data.is_video() {
        INDEX_COORDINATOR
            .execute_waiting(VideoTask::new(abstract_data))
            .await??;
    }

    Ok(())
}
