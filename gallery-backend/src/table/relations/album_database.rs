use crate::table::meta_album::{AlbumMetadataSchema, META_ALBUM_TABLE};
use crate::table::meta_image::{ImageMetadataSchema, META_IMAGE_TABLE};
use crate::table::meta_video::{META_VIDEO_TABLE, VideoMetadataSchema};
use crate::table::object::{OBJECT_TABLE, ObjectSchema};
use anyhow::Result;
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use redb::{ReadableTable, TableDefinition, WriteTransaction};
use std::collections::{HashMap, HashSet};

// 正向關聯: (AlbumId, Hash) -> ()
pub const ALBUM_ITEMS_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("rel_album_items");

// 反向關聯: (Hash, AlbumId) -> ()
pub const ITEM_ALBUMS_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("rel_item_albums");

pub struct AlbumDatabase;

impl AlbumDatabase {
    /// 新增項目到相簿，並自動更新統計數據 (模擬 Trigger)
    pub fn add_item(txn: &mut WriteTransaction, album_id: &str, hash: &str) -> Result<()> {
        // 1. 寫入雙向關聯
        {
            let mut forward = txn.open_table(ALBUM_ITEMS_TABLE)?;
            forward.insert((album_id, hash), &())?;
        }
        {
            let mut reverse = txn.open_table(ITEM_ALBUMS_TABLE)?;
            reverse.insert((hash, album_id), &())?;
        }

        // 2. 觸發統計更新
        Self::update_album_stats(txn, album_id)?;

        Ok(())
    }

    /// 從相簿移除項目，並自動更新統計數據
    pub fn remove_item(txn: &mut WriteTransaction, album_id: &str, hash: &str) -> Result<()> {
        // 1. 移除雙向關聯
        {
            let mut forward = txn.open_table(ALBUM_ITEMS_TABLE)?;
            forward.remove((album_id, hash))?;
        }
        {
            let mut reverse = txn.open_table(ITEM_ALBUMS_TABLE)?;
            reverse.remove((hash, album_id))?;
        }

        // 2. 觸發統計更新
        Self::update_album_stats(txn, album_id)?;

        Ok(())
    }

    /// 核心邏輯：重新計算相簿統計數據 (Count, Size, Time Range, Cover)
    fn update_album_stats(txn: &mut WriteTransaction, album_id: &str) -> Result<()> {
        // A. 讀取目前的 Album Metadata
        let mut album_table = txn.open_table(META_ALBUM_TABLE)?;
        let album_data = match album_table.get(album_id)? {
            Some(access) => access.value().to_vec(),
            None => return Ok(()), // 相簿不存在，忽略
        };
        let mut meta_album: AlbumMetadataSchema = bitcode::decode(&album_data)?;

        // B. 準備讀取關聯表
        let album_items = txn.open_table(ALBUM_ITEMS_TABLE)?;
        let object_table = txn.open_table(OBJECT_TABLE)?;
        let image_table = txn.open_table(META_IMAGE_TABLE)?;
        let video_table = txn.open_table(META_VIDEO_TABLE)?;

        // C. 掃描該相簿下的所有 Item
        // Range Scan: (album_id, "") .. (album_id, MAX)
        let start = (album_id, "");
        let end = (album_id, "\u{ffff}");
        let iter = album_items.range(start..=end)?;

        let mut count: usize = 0;
        let mut total_size: u64 = 0;
        let mut min_time: Option<i64> = None;
        let mut max_time: Option<i64> = None;
        let mut candidates_for_cover: Vec<(i64, String)> = Vec::new();

        for entry in iter {
            let ((_, hash), _) = entry?;
            let hash_str = hash;

            // 讀取 Object 基本資訊 (CreatedTime)
            if let Some(obj_bytes) = object_table.get(hash_str)? {
                let obj: ObjectSchema = bitcode::decode(obj_bytes.value())?;
                let c_time = obj.created_time;

                // 更新時間範圍
                min_time = Some(min_time.map_or(c_time, |t| t.min(c_time)));
                max_time = Some(max_time.map_or(c_time, |t| t.max(c_time)));

                // 讀取 Size (嘗試 Image 或 Video)
                let mut size = 0;
                if let Some(img_bytes) = image_table.get(hash_str)? {
                    let img: ImageMetadataSchema = bitcode::decode(img_bytes.value())?;
                    size = img.size;
                } else if let Some(vid_bytes) = video_table.get(hash_str)? {
                    let vid: VideoMetadataSchema = bitcode::decode(vid_bytes.value())?;
                    size = vid.size;
                }

                count += 1;
                total_size += size;
                candidates_for_cover.push((c_time, hash_str.to_string()));
            }
        }

        // D. 決定 Cover
        let current_cover_valid = if let Some(ref current) = meta_album.cover {
            candidates_for_cover
                .iter()
                .any(|(_, hash)| hash == current.as_str())
        } else {
            false
        };

        if !current_cover_valid {
            // 找時間最早的當封面
            candidates_for_cover.sort_by_key(|k| k.0);
            if let Some((_, first_hash)) = candidates_for_cover.first() {
                meta_album.cover = Some(ArrayString::from(first_hash).unwrap_or_default());
            } else {
                meta_album.cover = None;
            }
        }

        // E. 更新並寫回
        meta_album.item_count = count;
        meta_album.item_size = total_size;
        meta_album.start_time = min_time;
        meta_album.end_time = max_time;
        // 更新 LastModifiedTime
        meta_album.last_modified_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let serialized = bitcode::encode(&meta_album);
        album_table.insert(album_id, serialized.as_slice())?;

        Ok(())
    }

    /// 讀取：某個項目屬於哪些相簿 (反向索引)
    pub fn fetch_albums(
        txn: &redb::ReadTransaction,
        hash: &str,
    ) -> Result<HashSet<ArrayString<64>>> {
        let table = txn.open_table(ITEM_ALBUMS_TABLE)?;
        let start = (hash, "");
        let end = (hash, "\u{ffff}");
        let iter = table.range(start..=end)?;

        let mut albums = HashSet::new();
        for entry in iter {
            let ((_, album_id), _) = entry?;
            if let Ok(aid) = ArrayString::from(album_id) {
                albums.insert(aid);
            }
        }
        Ok(albums)
    }

    /// 讀取：批次取得所有關聯 (解決 N+1)
    pub fn fetch_all_albums(
        txn: &redb::ReadTransaction,
    ) -> Result<HashMap<ArrayString<64>, HashSet<ArrayString<64>>>> {
        let table = txn.open_table(ITEM_ALBUMS_TABLE)?;
        let iter = table.range::<(&str, &str)>(..)?;
        let mut map: HashMap<ArrayString<64>, HashSet<ArrayString<64>>> = HashMap::new();

        for entry in iter {
            let ((hash, album_id), _) = entry?;
            if let (Ok(h), Ok(a)) = (ArrayString::from(hash), ArrayString::from(album_id)) {
                map.entry(h).or_default().insert(a);
            }
        }
        Ok(map)
    }
}
