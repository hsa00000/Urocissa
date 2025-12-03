use std::collections::HashSet;

use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::public::db::tree::TREE;
use crate::table::album::AlbumCombined;
use crate::table::database::MediaCombined;

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
    pub schema: crate::table::database::DatabaseSchema, // 保留舊的用於兼容
    pub album: HashSet<ArrayString<64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)] // 這很重要，讓 JSON 輸出時不會多一層 Key，直接根據內容判斷
pub enum AbstractData {
    Media(MediaCombined),
    Album(AlbumCombined),
    #[serde(rename = "Database")]
    Database(Database), // 保留舊的用於向後兼容
}

impl AbstractData {
    pub fn compute_timestamp(self: &Self) -> i64 {
        match self {
            AbstractData::Media(media) => match media {
                MediaCombined::Image(i) => i.object.created_time,
                MediaCombined::Video(v) => v.object.created_time,
            },
            AbstractData::Album(album) => album.object.created_time,
            AbstractData::Database(database) => database.schema.timestamp_ms,
        }
    }
    pub fn hash(self: &Self) -> ArrayString<64> {
        match self {
            AbstractData::Media(media) => match media {
                MediaCombined::Image(i) => i.object.id,
                MediaCombined::Video(v) => v.object.id,
            },
            AbstractData::Album(album) => album.object.id,
            AbstractData::Database(database) => database.schema.hash,
        }
    }
    pub fn width(self: &Self) -> u32 {
        match self {
            AbstractData::Media(media) => match media {
                MediaCombined::Image(i) => i.metadata.width,
                MediaCombined::Video(v) => v.metadata.width,
            },
            AbstractData::Database(database) => database.schema.width,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn height(self: &Self) -> u32 {
        match self {
            AbstractData::Media(media) => match media {
                MediaCombined::Image(i) => i.metadata.height,
                MediaCombined::Video(v) => v.metadata.height,
            },
            AbstractData::Database(database) => database.schema.height,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn tag(self: &Self) -> Option<HashSet<String>> {
        match self {
            AbstractData::Database(database) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT tag FROM tag_databases WHERE hash = ?")
                    .unwrap();
                let tag_iter = stmt
                    .query_map([database.schema.hash.as_str()], |row| {
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
            AbstractData::Media(media) => {
                // 媒體的 tag 從關聯表讀取，類似 Database
                let conn = TREE.get_connection().unwrap();
                let id = match media {
                    MediaCombined::Image(i) => &i.object.id,
                    MediaCombined::Video(v) => &v.object.id,
                };
                let mut stmt = conn
                    .prepare("SELECT tag FROM tag_databases WHERE hash = ?")
                    .unwrap();
                let tag_iter = stmt
                    .query_map([id.as_str()], |row| {
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
            AbstractData::Album(album) => Some(album.metadata.tag.clone()),
        }
    }
    pub fn alias(self: &Self) -> Vec<FileModify> {
        match self {
            AbstractData::Media(media) => {
                let conn = TREE.get_connection().unwrap();
                let id = match media {
                    MediaCombined::Image(i) => &i.object.id,
                    MediaCombined::Video(v) => &v.object.id,
                };
                let mut stmt = conn
                    .prepare("SELECT file, modified, scan_time FROM database_alias WHERE hash = ? ORDER BY scan_time DESC")
                    .unwrap();
                let alias_iter = stmt
                    .query_map([id.as_str()], |row| {
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
                    .query_map([database.schema.hash.as_str()], |row| {
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
            AbstractData::Media(media) => {
                let conn = TREE.get_connection().unwrap();
                let id = match media {
                    MediaCombined::Image(i) => &i.object.id,
                    MediaCombined::Video(v) => &v.object.id,
                };
                let mut stmt = conn
                    .prepare("SELECT tag, value FROM database_exif WHERE hash = ?")
                    .unwrap();
                let exif_iter = stmt
                    .query_map([id.as_str()], |row| {
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
                    .query_map([database.schema.hash.as_str()], |row| {
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
            AbstractData::Media(media) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                let hash = match media {
                    MediaCombined::Image(i) => i.object.id,
                    MediaCombined::Video(v) => v.object.id,
                };
                ClaimsHash::new(hash, timestamp, allow_original).encode()
            }
            AbstractData::Database(db) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(db.schema.hash, timestamp, allow_original).encode()
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
