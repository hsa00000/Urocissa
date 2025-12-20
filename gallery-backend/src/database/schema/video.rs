use crate::database::schema::meta_video::{META_VIDEO_TABLE, VideoMetadataSchema};
use crate::database::schema::object::{OBJECT_TABLE, ObjectSchema, ObjectType};
use crate::database::schema::relations::album_database::AlbumDatabase;
use crate::database::schema::relations::database_exif::DatabaseExif;
use crate::database::schema::relations::tag_database::TagDatabase;
use anyhow::Result;
use arrayvec::ArrayString;
use redb::{ReadTransaction, ReadableTableMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: VideoMetadataSchema,
    #[serde(default)]
    pub albums: HashSet<ArrayString<64>>,
    #[serde(default, rename = "exifVec")]
    pub exif_vec: BTreeMap<String, String>,
}

impl VideoCombined {
    pub fn get_by_id(txn: &ReadTransaction, id: impl AsRef<str>) -> Result<Self> {
        let id = id.as_ref();
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let meta_table = txn.open_table(META_VIDEO_TABLE)?;

        let obj_bytes = obj_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Object not found"))?;
        let mut object: ObjectSchema = bitcode::decode(obj_bytes.value())?;

        let meta_bytes = meta_table
            .get(id)?
            .ok_or_else(|| anyhow::anyhow!("Video Metadata not found"))?;
        let metadata: VideoMetadataSchema = bitcode::decode(meta_bytes.value())?;

        let albums = AlbumDatabase::fetch_albums(txn, id)?;
        object.tags = TagDatabase::fetch_tags(txn, id)?;
        let exif_vec = DatabaseExif::fetch_exif(txn, id)?;

        Ok(Self {
            object,
            metadata,
            albums,
            exif_vec,
        })
    }

    pub fn get_raw_entries(txn: &ReadTransaction) -> Result<Vec<Self>> {
        let obj_table = txn.open_table(OBJECT_TABLE)?;
        let meta_table = txn.open_table(META_VIDEO_TABLE)?;

        // 優化 1: 預先分配記憶體，避免 Vec 擴容
        let count = meta_table.len()?;
        let mut videos = Vec::with_capacity(count as usize);

        for entry in meta_table.range::<&str>(..)? {
            let (id, meta_val) = entry?;
            let id_str = id.value();

            // Redb 優化: 如果 Object 和 Meta 是一對一且一致的，這裡的 get 還是必要的
            // 如果能確保資料完整性，這裡的 unwrap 可以視情況優化錯誤處理
            if let Some(obj_val) = obj_table.get(id_str)? {
                let object: ObjectSchema = bitcode::decode(obj_val.value())?;

                // Double Check 類型 (雖然從 MetaTable 讀取應該都是 Video)
                if object.obj_type == ObjectType::Video {
                    let metadata: VideoMetadataSchema = bitcode::decode(meta_val.value())?;

                    videos.push(Self {
                        object,
                        metadata,
                        // 關鍵：這裡先給空值，稍後在並行階段填入
                        albums: HashSet::new(),
                        exif_vec: BTreeMap::new(),
                    });
                }
            }
        }
        Ok(videos)
    }

    pub fn get_all(txn: &ReadTransaction) -> Result<Vec<Self>> {
        let mut videos = Self::get_raw_entries(txn)?;

        if videos.is_empty() {
            return Ok(videos);
        }

        let album_map = AlbumDatabase::fetch_all_albums(txn)?;
        let tag_map = TagDatabase::fetch_all_tags(txn)?;
        let exif_map = DatabaseExif::fetch_all_exif(txn)?;

        for video in &mut videos {
            if let Some(albums) = album_map.get(&video.object.id) {
                video.albums = albums.clone();
            }
            if let Some(tags) = tag_map.get(&video.object.id) {
                video.object.tags = tags.clone();
            }
            if let Some(exif) = exif_map.get(&video.object.id) {
                video.exif_vec = exif.clone();
            }
        }

        Ok(videos)
    }
}
