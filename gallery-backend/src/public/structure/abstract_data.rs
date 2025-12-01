use std::collections::HashSet;

use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::public::db::tree::TREE;
use crate::table::album::AlbumSchema;
use crate::table::database::DatabaseSchema;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbstractData {
    #[serde(rename = "Database")]
    DatabaseSchema(DatabaseSchema),
    Album(AlbumSchema),
}

impl AbstractData {
    pub fn compute_timestamp(self: &Self) -> i64 {
        match self {
            AbstractData::DatabaseSchema(database) => database.timestamp_ms,
            AbstractData::Album(album) => album.created_time as i64,
        }
    }
    pub fn hash(self: &Self) -> ArrayString<64> {
        match self {
            AbstractData::DatabaseSchema(database) => database.hash,
            AbstractData::Album(album) => album.id,
        }
    }
    pub fn width(self: &Self) -> u32 {
        match self {
            AbstractData::DatabaseSchema(database) => database.width,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn height(self: &Self) -> u32 {
        match self {
            AbstractData::DatabaseSchema(database) => database.height,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn tag(self: &Self) -> Option<HashSet<String>> {
        match self {
            AbstractData::DatabaseSchema(database) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT tag FROM tag_databases WHERE hash = ?")
                    .unwrap();
                let tag_iter = stmt
                    .query_map([database.hash.as_str()], |row| {
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
            AbstractData::Album(album) => Some(album.tag.clone()),
        }
    }
    pub fn alias(self: &Self) -> Vec<FileModify> {
        match self {
            AbstractData::DatabaseSchema(database) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT file, modified, scan_time FROM database_alias WHERE hash = ? ORDER BY scan_time DESC")
                    .unwrap();
                let alias_iter = stmt
                    .query_map([database.hash.as_str()], |row| {
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
            AbstractData::DatabaseSchema(database) => {
                let conn = TREE.get_connection().unwrap();
                let mut stmt = conn
                    .prepare("SELECT tag, value FROM database_exif WHERE hash = ?")
                    .unwrap();
                let exif_iter = stmt
                    .query_map([database.hash.as_str()], |row| {
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

    pub fn generate_token(&self, timestamp: u128) -> String {
        match self {
            AbstractData::DatabaseSchema(db) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(db.hash, timestamp, false).encode()
            }
            AbstractData::Album(album) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                // If the album has a cover, we must sign the cover hash
                // because the frontend will use this token to request the cover image file.
                // If there is no cover, we fallback to ID (though no image will be fetched).
                let hash = album.cover.unwrap_or(album.id);
                ClaimsHash::new(hash, timestamp, false).encode()
            }
        }
    }

    pub fn with_tag(self, timestamp: u128) -> AbstractDataWithTag {
        let tag = self.tag();
        let exif_vec = self.exif_vec().unwrap_or_default();
        let alias = self.alias();
        let token = self.generate_token(timestamp);
        AbstractDataWithTag {
            data: self,
            tag,
            alias,
            token,
            exif_vec,
        }
    }
}

impl From<DatabaseSchema> for AbstractData {
    fn from(database: DatabaseSchema) -> Self {
        AbstractData::DatabaseSchema(database)
    }
}

impl From<AlbumSchema> for AbstractData {
    fn from(album: AlbumSchema) -> Self {
        AbstractData::Album(album)
    }
}
