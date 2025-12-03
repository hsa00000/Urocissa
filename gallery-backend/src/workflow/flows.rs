//! Business flows module - high-level workflow orchestration
//!
//! This module contains the main business logic flows that coordinate
//! multiple tasks together to accomplish complete operations.

use anyhow::Result;
use arrayvec::ArrayString;
use log::warn;
use path_clean::PathClean;
use std::path::PathBuf;

use crate::workflow::{
    tasks::{
        BATCH_COORDINATOR, INDEX_COORDINATOR,
        actor::{
            copy::CopyTask, deduplicate::DeduplicateTask, delete_in_update::DeleteTask,
            hash::HashTask, index::IndexTask, open_file::OpenFileTask, video::VideoTask,
        },
        batcher::flush_tree::FlushTreeTask,
    },
    types::try_acquire,
};

/// Main indexing flow for file watcher events
///
/// This flow handles the complete lifecycle of importing a new media file:
/// 1. Open file with retry logic
/// 2. Compute Blake3 hash
/// 3. Check for duplicates
/// 4. Copy to imported directory
/// 5. Process metadata (image/video)
/// 6. Flush to database
/// 7. Cleanup source file (if in upload directory)
/// 8. Compress video (if applicable)
pub async fn index_for_watch(
    path: PathBuf,
    presigned_album_id_opt: Option<ArrayString<64>>,
) -> Result<()> {
    let path = path.clean();

    // Step 1: Open file
    let file = INDEX_COORDINATOR
        .execute_waiting(OpenFileTask::new(path.clone()))
        .await??;

    // Step 2: Compute hash
    let hash = INDEX_COORDINATOR
        .execute_waiting(HashTask::new(file))
        .await??;

    // Step 3: Acquire processing guard to prevent duplicate processing
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

    // Step 4: Check for duplicates
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
            // File already exists, just delete the source
            INDEX_COORDINATOR.execute_detached(DeleteTask::new(path));
            return Ok(());
        }
    };

    // Step 5: Copy file to imported directory
    let copied_database = INDEX_COORDINATOR
        .execute_waiting(CopyTask::new(path.clone(), database.clone()))
        .await??;
    database = copied_database;

    // Step 6: Process metadata
    let (index_task, flush_task_from_index) = INDEX_COORDINATOR
        .execute_waiting(IndexTask::new(path.clone(), database.clone()))
        .await??;

    // Update database schema from index_task
    database = index_task.into();

    // Step 7: Combine all flush operations and persist
    let mut all_operations = vec![];
    all_operations.extend(flush_task_from_index.operations);
    all_operations.extend(dedup_flush.operations);

    BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask {
        operations: all_operations,
    });

    // Step 8: Cleanup source file
    INDEX_COORDINATOR.execute_detached(DeleteTask::new(PathBuf::from(&path)));

    // Step 9: Compress video if needed
    if database.ext_type() == "video" {
        INDEX_COORDINATOR
            .execute_waiting(VideoTask::new(database.clone()))
            .await??;
    }

    Ok(())
}
