use crate::background::actors::BATCH_COORDINATOR;
use crate::background::batchers::expire::UpdateExpireTask;
use crate::background::processors::transitor::get_current_timestamp_u64;
use crate::database::ops::tree::TREE;
use crate::database::schema::image::ImageCombined;
use crate::database::schema::relations::album_database::AlbumDatabase;
use crate::database::schema::relations::database_exif::DatabaseExif;
use crate::database::schema::relations::tag_database::TagDatabase;
use crate::database::schema::video::VideoCombined;
use crate::models::entity::abstract_data::AbstractData;
use anyhow::Result;
use arrayvec::ArrayString;
use log::error;
use mini_executor::BatchTask;
use rayon::prelude::*;
use redb::ReadableDatabase;
use std::collections::{BTreeMap, HashMap, HashSet};
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

    // 定義 helper 避免重複代碼
    let fetch_aux_data = || -> Result<(HashMap<ArrayString<64>, HashSet<ArrayString<64>>>, HashMap<ArrayString<64>, HashSet<String>>, HashMap<ArrayString<64>, BTreeMap<String, String>>)> {
        let txn = TREE.in_disk.begin_read()?;
        // 這裡也可以內部並行，但考慮到 map 建構開銷，放在一起也可以
        let albums = AlbumDatabase::fetch_all_albums(&txn)?;
        let tags = TagDatabase::fetch_all_tags(&txn)?;
        let exif = DatabaseExif::fetch_all_exif(&txn)?;
        Ok((albums, tags, exif))
    };

    // 2. Rayon 並行讀取 (Fetch Phase)
    let ((images_res, videos_res), aux_res) = rayon::join(
        || {
            rayon::join(
                || -> Result<Vec<ImageCombined>> {
                    let txn = TREE.in_disk.begin_read()?;
                    ImageCombined::get_raw_entries(&txn)
                },
                || -> Result<Vec<VideoCombined>> {
                    let txn = TREE.in_disk.begin_read()?;
                    VideoCombined::get_raw_entries(&txn)
                },
            )
        },
        || fetch_aux_data(),
    );

    // 處理錯誤 (Fail fast)
    let (images, videos) = (images_res?, videos_res?);
    let (album_map, tag_map, exif_map) = aux_res?;
    let mut images = images;
    let mut videos = videos;

    // 3. 並行組裝 (Merge Phase)
    // 我們使用 rayon 的 par_iter_mut 修改 images 和 videos
    // 這裡需要 map 是唯讀引用的 (&Map)，這在 rayon 中是線程安全的

    let merge_image_job = || {
        images.par_iter_mut().for_each(|img| {
            let id = &img.object.id;
            // 填充相簿
            if let Some(a) = album_map.get(id) {
                img.albums = a.clone(); // 這裡 Clone HashSet<String> 比 IO 快得多
            }
            // 填充標籤
            if let Some(t) = tag_map.get(id) {
                img.object.tags = t.clone();
            }
            // 填充並過濾 EXIF (同時做過濾，省一次遍歷)
            if let Some(e) = exif_map.get(id) {
                // 這裡實作你的 ALLOWED_KEYS 過濾邏輯
                img.exif_vec = e
                    .iter()
                    .filter(|(k, _)| ALLOWED_KEYS.contains(k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
            }
        });
    };

    let merge_video_job = || {
        videos.par_iter_mut().for_each(|vid| {
            let id = &vid.object.id;
            if let Some(a) = album_map.get(id) {
                vid.albums = a.clone();
            }
            if let Some(t) = tag_map.get(id) {
                vid.object.tags = t.clone();
            }
            if let Some(e) = exif_map.get(id) {
                vid.exif_vec = e
                    .iter()
                    .filter(|(k, _)| ALLOWED_KEYS.contains(k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
            }
        });
    };

    // 同時執行組裝
    rayon::join(merge_image_job, merge_video_job);

    // 4. 讀取相簿 (因為相簿邏輯比較獨立，可以跟上面的並行，但為了程式碼清晰先放這)
    // 如果相簿數量多，也可以放入上面的 rayon::join
    let albums_vec = {
        let rows = TREE.read_albums()?;
        rows.into_par_iter()
            .map(AbstractData::Album)
            .collect::<Vec<_>>()
    };

    // 5. 合併資料
    let mut result_vec = Vec::with_capacity(images.len() + videos.len() + albums_vec.len());

    // 將 Images 和 Videos 轉為 AbstractData
    result_vec.par_extend(images.into_par_iter().map(AbstractData::Image));
    result_vec.par_extend(videos.into_par_iter().map(AbstractData::Video));
    result_vec.extend(albums_vec);

    // 6. 並行排序 (Sort)
    // Unstable sort 更快且省記憶體
    result_vec.par_sort_unstable_by(|a, b| b.compute_timestamp().cmp(&a.compute_timestamp()));

    // 7. 更新全域快取
    *TREE.in_memory.write().unwrap() = result_vec;

    // ... 後續通知邏輯
    BATCH_COORDINATOR.execute_batch_detached(UpdateExpireTask);

    let current_timestamp = get_current_timestamp_u64();
    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "In-memory cache updated ({}).", current_timestamp);

    Ok(())
}
