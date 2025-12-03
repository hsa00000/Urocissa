use std::collections::HashSet;

use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::public::db::tree::TREE;
use crate::table::album::AlbumCombined;
use crate::table::image::ImageCombined;
use crate::table::video::VideoCombined;

use super::database::file_modify::FileModify;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbstractDataWithTag {
    pub data: AbstractData,
    pub tag: Option<HashSet<String>>,
    pub alias: Vec<FileModify>,
    pub token: String,
    pub exif_vec: BTreeMap<String, String>,
}

/// Database: 記憶體中的資料庫物件，組合了 Schema 與關聯資訊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    #[serde(flatten)]
    pub media: MediaWithAlbum,
    pub album: HashSet<ArrayString<64>>,
}

impl Database {
    /// 獲取 hash
    pub fn hash(&self) -> ArrayString<64> {
        match &self.media {
            MediaWithAlbum::Image(img) => img.object.id,
            MediaWithAlbum::Video(vid) => vid.object.id,
        }
    }

    /// 獲取 timestamp
    pub fn timestamp_ms(&self) -> i64 {
        match &self.media {
            MediaWithAlbum::Image(img) => img.object.created_time,
            MediaWithAlbum::Video(vid) => vid.object.created_time,
        }
    }

    /// 獲取 ext_type
    pub fn ext_type(&self) -> &str {
        match &self.media {
            MediaWithAlbum::Image(_) => "image",
            MediaWithAlbum::Video(_) => "video",
        }
    }

    /// 獲取 size
    pub fn size(&self) -> u64 {
        match &self.media {
            MediaWithAlbum::Image(img) => img.metadata.size,
            MediaWithAlbum::Video(vid) => vid.metadata.size,
        }
    }

    /// 獲取 width
    pub fn width(&self) -> u32 {
        match &self.media {
            MediaWithAlbum::Image(img) => img.metadata.width,
            MediaWithAlbum::Video(vid) => vid.metadata.width,
        }
    }

    /// 獲取 height
    pub fn height(&self) -> u32 {
        match &self.media {
            MediaWithAlbum::Image(img) => img.metadata.height,
            MediaWithAlbum::Video(vid) => vid.metadata.height,
        }
    }

    /// 獲取 ext
    pub fn ext(&self) -> &str {
        match &self.media {
            MediaWithAlbum::Image(img) => &img.metadata.ext,
            MediaWithAlbum::Video(vid) => &vid.metadata.ext,
        }
    }

    /// 獲取 thumbhash (mutable)
    pub fn thumbhash(&self) -> Vec<u8> {
        match &self.media {
            MediaWithAlbum::Image(img) => img.object.thumbhash.clone().unwrap_or_default(),
            MediaWithAlbum::Video(vid) => vid.object.thumbhash.clone().unwrap_or_default(),
        }
    }

    /// 設置 thumbhash
    pub fn set_thumbhash(&mut self, thumbhash: Vec<u8>) {
        match &mut self.media {
            MediaWithAlbum::Image(img) => img.object.thumbhash = Some(thumbhash),
            MediaWithAlbum::Video(vid) => vid.object.thumbhash = Some(thumbhash),
        }
    }

    /// 獲取 phash (mutable)
    pub fn phash(&self) -> Vec<u8> {
        match &self.media {
            MediaWithAlbum::Image(img) => img.metadata.phash.clone().unwrap_or_default(),
            MediaWithAlbum::Video(_) => Vec::new(), // Video 沒有 phash
        }
    }

    /// 設置 phash
    pub fn set_phash(&mut self, phash: Vec<u8>) {
        match &mut self.media {
            MediaWithAlbum::Image(img) => img.metadata.phash = Some(phash),
            MediaWithAlbum::Video(_) => {} // Video 沒有 phash
        }
    }

    /// 獲取 pending
    pub fn pending(&self) -> bool {
        match &self.media {
            MediaWithAlbum::Image(img) => img.object.pending,
            MediaWithAlbum::Video(vid) => vid.object.pending,
        }
    }

    /// 設置 pending
    pub fn set_pending(&mut self, pending: bool) {
        match &mut self.media {
            MediaWithAlbum::Image(img) => img.object.pending = pending,
            MediaWithAlbum::Video(vid) => vid.object.pending = pending,
        }
    }

    /// 獲取 imported_path
    pub fn imported_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.imported_path_string())
    }

    /// 獲取 imported_path_string
    pub fn imported_path_string(&self) -> String {
        format!(
            "./object/imported/{}/{}.{}",
            &self.hash()[0..2],
            self.hash(),
            self.ext()
        )
    }

    /// 獲取 compressed_path_string
    pub fn compressed_path_string(&self) -> String {
        if self.ext_type() == "image" {
            format!("./object/compressed/{}/{}.jpg", &self.hash()[0..2], self.hash())
        } else {
            format!("./object/compressed/{}/{}.mp4", &self.hash()[0..2], self.hash())
        }
    }
}

/// 媒體與相簿關聯的組合
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaWithAlbum {
    Image(ImageCombined),
    Video(VideoCombined),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)] // 這很重要，讓 JSON 輸出時不會多一層 Key，直接根據內容判斷
pub enum AbstractData {
    Image(ImageCombined),
    Video(VideoCombined),
    Album(AlbumCombined),
    #[serde(rename = "Database")]
    Database(Database), // 保留舊的用於向後兼容
}

impl AbstractData {
    pub fn compute_timestamp(self: &Self) -> i64 {
        match self {
            AbstractData::Image(i) => i.object.created_time,
            AbstractData::Video(v) => v.object.created_time,
            AbstractData::Album(album) => album.object.created_time,
            AbstractData::Database(database) => database.timestamp_ms(),
        }
    }
    pub fn hash(self: &Self) -> ArrayString<64> {
        match self {
            AbstractData::Image(i) => i.object.id,
            AbstractData::Video(v) => v.object.id,
            AbstractData::Album(album) => album.object.id,
            AbstractData::Database(database) => database.hash(),
        }
    }
    pub fn width(self: &Self) -> u32 {
        match self {
            AbstractData::Image(i) => i.metadata.width,
            AbstractData::Video(v) => v.metadata.width,
            AbstractData::Database(database) => database.width(),
            AbstractData::Album(_) => 300,
        }
    }
    pub fn height(self: &Self) -> u32 {
        match self {
            AbstractData::Image(i) => i.metadata.height,
            AbstractData::Video(v) => v.metadata.height,
            AbstractData::Database(database) => database.height(),
            AbstractData::Album(_) => 300,
        }
    }
    pub fn tag(self: &Self) -> Option<HashSet<String>> {
        // 統一邏輯：所有類型的 tag 都從 tag_databases 關聯表讀取
        let hash = self.hash();
        let conn = TREE.get_connection().unwrap();
        let mut stmt = conn
            .prepare("SELECT tag FROM tag_databases WHERE hash = ?")
            .unwrap();
        let tag_iter = stmt
            .query_map([hash.as_str()], |row| {
                let tag: String = row.get(0)?;
                Ok(tag)
            })
            .unwrap();
        let mut tags = HashSet::new();
        for tag_result in tag_iter {
            if let Ok(tag) = tag_result {
                tags.insert(tag);
            }
        }
        Some(tags)
    }
    pub fn alias(self: &Self) -> Vec<FileModify> {
        match self {
            AbstractData::Image(i) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT file, modified, scan_time FROM database_alias WHERE hash = ? ORDER BY scan_time DESC")
                    .unwrap();
                let alias_iter = stmt
                    .query_map([i.object.id.as_str()], |row| {
                        Ok(FileModify {
                            file: row.get(0)?,
                            modified: row.get::<_, i64>(1)? as u128,
                            scan_time: row.get::<_, i64>(2)? as u128,
                        })
                    })
                    .unwrap();
                let aliases: Vec<FileModify> = alias_iter.filter_map(|r| r.ok()).collect();
                aliases
            }
            AbstractData::Video(v) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT file, modified, scan_time FROM database_alias WHERE hash = ? ORDER BY scan_time DESC")
                    .unwrap();
                let alias_iter = stmt
                    .query_map([v.object.id.as_str()], |row| {
                        Ok(FileModify {
                            file: row.get(0)?,
                            modified: row.get::<_, i64>(1)? as u128,
                            scan_time: row.get::<_, i64>(2)? as u128,
                        })
                    })
                    .unwrap();
                let aliases: Vec<FileModify> = alias_iter.filter_map(|r| r.ok()).collect();
                aliases
            }
            AbstractData::Database(database) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT file, modified, scan_time FROM database_alias WHERE hash = ? ORDER BY scan_time DESC")
                    .unwrap();
                let alias_iter = stmt
                    .query_map([database.hash().as_str()], |row| {
                        Ok(FileModify {
                            file: row.get(0)?,
                            modified: row.get::<_, i64>(1)? as u128,
                            scan_time: row.get::<_, i64>(2)? as u128,
                        })
                    })
                    .unwrap();
                let aliases: Vec<FileModify> = alias_iter.filter_map(|r| r.ok()).collect();
                aliases
            }
            AbstractData::Album(_) => vec![],
        }
    }
    pub fn exif_vec(self: &Self) -> Option<BTreeMap<String, String>> {
        match self {
            AbstractData::Image(i) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT tag, value FROM database_exif WHERE hash = ?")
                    .unwrap();
                let exif_iter = stmt
                    .query_map([i.object.id.as_str()], |row| {
                        let tag: String = row.get(0)?;
                        let value: String = row.get(1)?;
                        Ok((tag, value))
                    })
                    .unwrap();
                let mut exif_map = BTreeMap::new();
                for exif_result in exif_iter {
                    if let Ok((tag, value)) = exif_result {
                        exif_map.insert(tag, value);
                    }
                }
                Some(exif_map)
            }
            AbstractData::Video(v) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT tag, value FROM database_exif WHERE hash = ?")
                    .unwrap();
                let exif_iter = stmt
                    .query_map([v.object.id.as_str()], |row| {
                        let tag: String = row.get(0)?;
                        let value: String = row.get(1)?;
                        Ok((tag, value))
                    })
                    .unwrap();
                let mut exif_map = BTreeMap::new();
                for exif_result in exif_iter {
                    if let Ok((tag, value)) = exif_result {
                        exif_map.insert(tag, value);
                    }
                }
                Some(exif_map)
            }
            AbstractData::Database(database) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT tag, value FROM database_exif WHERE hash = ?")
                    .unwrap();
                let exif_iter = stmt
                    .query_map([database.hash().as_str()], |row| {
                        let tag: String = row.get(0)?;
                        let value: String = row.get(1)?;
                        Ok((tag, value))
                    })
                    .unwrap();
                let mut exif_map = BTreeMap::new();
                for exif_result in exif_iter {
                    if let Ok((tag, value)) = exif_result {
                        exif_map.insert(tag, value);
                    }
                }
                Some(exif_map)
            }
            AbstractData::Album(_) => None,
        }
    }

    pub fn generate_token(&self, timestamp: u128, allow_original: bool) -> String {
        match self {
            AbstractData::Image(i) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(i.object.id, timestamp, allow_original).encode()
            }
            AbstractData::Video(v) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(v.object.id, timestamp, allow_original).encode()
            }
            AbstractData::Database(db) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(db.hash(), timestamp, allow_original).encode()
            }
            AbstractData::Album(album) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                // If the album has a cover, we must sign the cover hash
                // because the frontend will use this token to request the cover image file.
                // If there is no cover, we fallback to ID (though no image will be fetched).
                let hash = album.metadata.cover.unwrap_or(album.object.id);
                ClaimsHash::new(hash, timestamp, allow_original).encode()
            }
        }
    }

    pub fn with_tag(self, timestamp: u128, allow_original: bool) -> AbstractDataWithTag {
        let tag = self.tag();
        let exif_vec = self.exif_vec().unwrap_or_default();
        let alias = self.alias();
        let token = self.generate_token(timestamp, allow_original);
        AbstractDataWithTag {
            data: self,
            tag,
            alias,
            token,
            exif_vec,
        }
    }
}

// 為了方便從 DatabaseSchema 轉換，但現在需要 album 資訊，所以不能直接 From<DatabaseSchema>
// 我們可以實作 From<Database>
impl From<Database> for AbstractData {
    fn from(database: Database) -> Self {
        AbstractData::Database(database)
    }
}
