use std::collections::HashSet;

use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::public::db::tree::TREE;
use crate::table::album::AlbumCombined;
use crate::table::image::ImageCombined;
use crate::table::video::VideoCombined;

use super::database::file_modify::FileModify;

#[derive(Debug, Clone)]
pub struct Database {
    pub media: AbstractData,
    pub album: HashSet<String>,
}

impl Database {
    pub fn imported_path(&self) -> PathBuf {
        match &self.media {
            AbstractData::Image(img) => img.imported_path(),
            AbstractData::Video(vid) => vid.imported_path(),
            AbstractData::Album(_) => PathBuf::new(), // or handle appropriately
        }
    }

    pub fn imported_path_string(&self) -> String {
        self.imported_path().to_string_lossy().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbstractDataWithTag {
    pub data: AbstractData,
    pub tag: Option<HashSet<String>>,
    pub alias: Vec<FileModify>,
    pub token: String,
    pub exif_vec: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)] // 這很重要，讓 JSON 輸出時不會多一層 Key，直接根據內容判斷
pub enum AbstractData {
    Image(ImageCombined),
    Video(VideoCombined),
    Album(AlbumCombined),
}

impl AbstractData {
    pub fn compute_timestamp(self: &Self) -> i64 {
        match self {
            AbstractData::Image(i) => i.object.created_time,
            AbstractData::Video(v) => v.object.created_time,
            AbstractData::Album(album) => album.object.created_time,
        }
    }
    pub fn hash(self: &Self) -> ArrayString<64> {
        match self {
            AbstractData::Image(i) => i.object.id,
            AbstractData::Video(v) => v.object.id,
            AbstractData::Album(album) => album.object.id,
        }
    }
    pub fn width(self: &Self) -> u32 {
        match self {
            AbstractData::Image(i) => i.metadata.width,
            AbstractData::Video(v) => v.metadata.width,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn height(self: &Self) -> u32 {
        match self {
            AbstractData::Image(i) => i.metadata.height,
            AbstractData::Video(v) => v.metadata.height,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn tag(&self) -> Option<&HashSet<String>> {
        match self {
            AbstractData::Image(i) => Some(&i.tags),
            AbstractData::Video(v) => Some(&v.tags),
            AbstractData::Album(a) => Some(&a.tags),
        }
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
        // 優化：這裡不再查詢 DB，而是直接複製內存中的 tags
        let tag = self.tag().cloned();
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
