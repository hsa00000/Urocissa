use crate::background::types::try_acquire;
use anyhow::{Result, anyhow};
use arrayvec::ArrayString;
use log::warn;
use path_clean::PathClean;
use std::path::PathBuf;
use tokio::task::spawn_blocking;

use crate::models::entity::abstract_data::AbstractData;
use crate::cli::tui::DASHBOARD;
use crate::utils::imported_path;
use crate::background::actors::{
    BATCH_COORDINATOR,
    INDEX_COORDINATOR,
        copy::CopyTask, deduplicate::DeduplicateTask, delete_in_update::DeleteTask,
 hash::HashTask,
    index::IndexTask, open_file::OpenFileTask, video::VideoTask,
};
use crate::background::batchers::flush_tree::{FlushOperation, FlushTreeTask}; // 確保引入 FlushOperation
// 引入 ExifSchema 以便建立寫入操作
use crate::database::schema::relations::database_exif::ExifSchema;

pub async fn index_workflow(
    path: impl Into<PathBuf>,
    presigned_album_id_opt: Option<ArrayString<64>>,
) -> Result<()> {
    let path = path.into().clean();

    // Step 1: Open file
    let file = match INDEX_COORDINATOR
        .execute_waiting(OpenFileTask::new(path.clone()))
        .await
    {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => return Err(e),
        Err(e) => return Err(anyhow!("Join error: {}", e)),
    };

    // Step 2: Calculate Hash
    let hash = match INDEX_COORDINATOR.execute_waiting(HashTask::new(file)).await {
        Ok(Ok(h)) => h,
        Ok(Err(e)) => return Err(e),
        Err(e) => return Err(anyhow!("Join error: {}", e)),
    };

    // Step 2: Acquire processing guard
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

    // Step 3: Deduplicate Check
    let result = match INDEX_COORDINATOR
        .execute_waiting(DeduplicateTask::new(
            path.clone(),
            hash,
            presigned_album_id_opt,
        ))
        .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            DASHBOARD.mark_failed(&hash);
            return Err(e);
        }
        Err(e) => {
            DASHBOARD.mark_failed(&hash);
            return Err(anyhow!("Join error: {}", e));
        }
    };

    let (mut data, dedup_ops) = match result {
        Some((d, ops)) => (d, ops),
        None => {
            // File exists, processed in DeduplicateTask, just delete source
            INDEX_COORDINATOR.execute_detached(DeleteTask::new(path));
            return Ok(());
        }
    };

    // Step 4: Copy file to imported directory
    let copied_data = match INDEX_COORDINATOR
        .execute_waiting(CopyTask::new(path.clone(), data.clone()))
        .await
    {
        Ok(Ok(d)) => d,
        Ok(Err(e)) => {
            DASHBOARD.mark_failed(&hash);
            return Err(e);
        }
        Err(e) => {
            DASHBOARD.mark_failed(&hash);
            return Err(anyhow!("Join error: {}", e));
        }
    };
    data = copied_data;

    // Step 5: Process metadata (in blocking thread)
    let data_clone = data.clone();
    let imported_path = match &data {
        AbstractData::Image(i) => imported_path(i.object.id, &i.metadata.ext),
        AbstractData::Video(v) => imported_path(v.object.id, &v.metadata.ext),
        _ => {
            DASHBOARD.mark_failed(&hash);
            return Err(anyhow!("Unsupported data type"));
        }
    };

    // 這裡我們會拿回更新後的 index_task，它包含了 process_image_info 解析出的 EXIF 資料
    let (updated_index_task, duration_opt) =
        match spawn_blocking(move || -> Result<(IndexTask, Option<f64>)> {
            let mut index_task = IndexTask::new(imported_path.clone(), data_clone.clone());
            let mut duration = None;

            match &data_clone {
                AbstractData::Image(_) => {
                    crate::background::processors::image::process_image_info(&mut index_task)?;
                }
                AbstractData::Video(_) => {
                    crate::background::processors::video::regenerate_metadata_for_video(
                        &mut index_task,
                    )?;
                    let d = crate::background::processors::video::video_duration(
                        &imported_path.to_string_lossy(),
                    )
                    .unwrap_or(0.0);
                    duration = Some(d);
                }
                _ => {}
            }
            Ok((index_task, duration))
        })
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                DASHBOARD.mark_failed(&hash);
                return Err(e);
            }
            Err(e) => {
                DASHBOARD.mark_failed(&hash);
                return Err(anyhow!("Spawn blocking error: {}", e));
            }
        };

    // Update data with processed results (width, height, etc.)
    data = updated_index_task.clone().into();

    // [新增]: 在這裡通知 TUI 任務進度已推進 (完成索引/處理)
    DASHBOARD.advance_task_state(&hash);

    if let (AbstractData::Video(vid), Some(d)) = (&mut data, duration_opt) {
        vid.metadata.duration = d;
    }

    // Step 6: Create Flush Operations
    let mut all_operations = vec![];

    // 1. 寫入主要物件資料 (Object, MetaImage/MetaVideo, Album關聯)
    all_operations.push(FlushOperation::InsertAbstractData(data.clone()));

    // 2. 寫入 Alias (從 deduplicate task 來的)
    all_operations.extend(dedup_ops);

    // 3. [FIXED] 寫入 EXIF 資料
    // 從 updated_index_task 中提取 exif_vec 並轉換為 InsertExif 操作
    let hash_string = data.hash().to_string();
    for (tag, value) in updated_index_task.exif_vec {
        all_operations.push(FlushOperation::InsertExif(ExifSchema {
            hash: hash_string.clone(),
            tag,
            value,
        }));
    }

    // 執行所有寫入操作
    BATCH_COORDINATOR.execute_batch_detached(FlushTreeTask {
        operations: all_operations,
    });

    // Step 7: Cleanup source file
    INDEX_COORDINATOR.execute_detached(DeleteTask::new(&path));

    // Step 8: Compress video if needed
    if let AbstractData::Video(_) = &data {
        match INDEX_COORDINATOR
            .execute_waiting(VideoTask::new(data.clone()))
            .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                DASHBOARD.mark_failed(&hash);
                return Err(e);
            }
            Err(e) => {
                DASHBOARD.mark_failed(&hash);
                return Err(anyhow!("Join error: {}", e));
            }
        };
    }

    Ok(())
}
