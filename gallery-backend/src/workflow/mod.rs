use crate::tasks::{
    BATCH_COORDINATOR, INDEX_COORDINATOR,
    actor::{
        copy::CopyTask, deduplicate::DeduplicateTask, delete_in_update::DeleteTask, hash::HashTask,
        index::IndexTask, open_file::OpenFileTask, video::VideoTask,
    },
    batcher::flush_tree::FlushTreeTask,
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

    let result = INDEX_COORDINATOR
        .execute_waiting(DeduplicateTask::new(
            path.clone(),
            hash,
            presigned_album_id_opt,
        ))
        .await??;

    let (mut database, dedup_flush) = match result {
        Some((db, flush_task)) => (db, flush_task),
        None => {
            INDEX_COORDINATOR.execute_detached(DeleteTask::new(path));
            return Ok(());
        }
    };

    database = INDEX_COORDINATOR
        .execute_waiting(CopyTask::new(path.clone(), database))
        .await??;
    let (database, flush_task_from_index) = INDEX_COORDINATOR
        .execute_waiting(IndexTask::new(path.clone(), database))
        .await??;

    // Combine all flush operations
    let mut all_operations = vec![];
    all_operations.extend(flush_task_from_index.operations);
    all_operations.extend(dedup_flush.operations);

    BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask {
        operations: all_operations,
    });

    INDEX_COORDINATOR.execute_detached(DeleteTask::new(PathBuf::from(&path)));
    if database.ext_type == "video" {
        INDEX_COORDINATOR
            .execute_waiting(VideoTask::new(database))
            .await??;
    }

    Ok(())
}
