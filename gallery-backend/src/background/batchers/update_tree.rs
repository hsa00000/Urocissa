use crate::background::actors::BATCH_COORDINATOR;
use crate::background::batchers::expire::UpdateExpireTask;
use crate::background::processors::transitor::get_current_timestamp_u64;
use crate::database::ops::tree::TREE;
use crate::models::entity::abstract_data::AbstractData;
use anyhow::Result;
use log::error;
use mini_executor::BatchTask;
use rayon::prelude::*;
use redb::ReadableDatabase;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Instant;

static ALLOWED_KEYS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "Make",
        "Model",
        "FNumber",
        "ExposureTime",
        "FocalLength",
        "PhotographicSensitivity",
        "DateTimeOriginal",
        "duration",
        "rotation",
    ]
    .iter()
    .cloned()
    .collect()
});

pub struct UpdateTreeTask;

impl BatchTask for UpdateTreeTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            if let Err(e) = update_tree_task() {
                error!("Error in update_tree_task: {}", e);
            }
        }
    }
}

fn update_tree_task() -> Result<()> {
    let start_time = Instant::now();

    // 定義一個 helper closure 來處理 EXIF 過濾，避免重複程式碼
    // 這是一個純函數，只負責過濾，不涉及 I/O
    let filter_exif = |mut exif_vec: std::collections::BTreeMap<String, String>| {
        exif_vec.retain(|k, _| ALLOWED_KEYS.contains(k.as_str()));
        exif_vec
    };

    // 使用 rayon::join 讓三個資料庫讀取任務「同時」進行
    // 注意：這取決於你的 DB 是否支援同時開啟多個 Read Transaction
    // 大多數嵌入式 DB (如 redb, sled, lmdb) 的 Read TXN 都是輕量且並發安全的
    let ((images, videos), albums) = rayon::join(
        || {
            rayon::join(
                || -> Result<Vec<AbstractData>> {
                    let txn = TREE.in_disk.begin_read()?; // 每個執行緒開啟自己的讀取交易
                    let raw_images = crate::database::schema::image::ImageCombined::get_all(&txn)?; // 假設這裡回傳 Vec<ImageCombined>

                    // 立即轉為並行迭代器進行過濾與封裝
                    Ok(raw_images
                        .into_par_iter()
                        .map(|mut img| {
                            // 在這裡直接過濾，省去後續的遍歷
                            img.exif_vec = filter_exif(img.exif_vec);
                            AbstractData::Image(img)
                        })
                        .collect())
                },
                || -> Result<Vec<AbstractData>> {
                    let txn = TREE.in_disk.begin_read()?;
                    let raw_videos = crate::database::schema::video::VideoCombined::get_all(&txn)?;

                    Ok(raw_videos
                        .into_par_iter()
                        .map(|mut video| {
                            video.exif_vec = filter_exif(video.exif_vec);
                            AbstractData::Video(video)
                        })
                        .collect())
                },
            )
        },
        || -> Result<Vec<AbstractData>> {
            let rows = TREE.read_albums()?;
            // 相簿沒有 EXIF，直接轉換
            Ok(rows.into_par_iter().map(AbstractData::Album).collect())
        },
    );

    // 處理 Result，若有錯誤提早返回
    let images_vec = images?;
    let videos_vec = videos?;
    let albums_vec = albums?;

    // 合併資料
    // 使用 reserve 避免多次記憶體重分配
    let mut abstract_data_vec =
        Vec::with_capacity(images_vec.len() + videos_vec.len() + albums_vec.len());
    abstract_data_vec.extend(images_vec);
    abstract_data_vec.extend(videos_vec);
    abstract_data_vec.extend(albums_vec);

    // 最後進行並行排序
    abstract_data_vec.par_sort_unstable_by(|a, b| {
        // 使用 unstable 排序通常比 stable 快，且記憶體開銷較小 (若你不需要穩定排序)
        b.compute_timestamp().cmp(&a.compute_timestamp())
    });

    // 更新記憶體快取
    *TREE.in_memory.write().unwrap() = abstract_data_vec;

    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = get_current_timestamp_u64();
    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "In-memory cache updated ({}).", current_timestamp);
    Ok(())
}
