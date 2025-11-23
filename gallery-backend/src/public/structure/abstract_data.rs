use std::collections::HashSet;

use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};

use crate::public::db::tree::TREE;

use super::{album::Album, database_struct::database::definition::Database};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractDataWithTag {
    pub data: AbstractData,
    pub tag: Option<HashSet<String>>,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbstractData {
    Database(Database),
    Album(Album),
}

impl AbstractData {
    pub fn compute_timestamp(self: &Self) -> i64 {
        match self {
            AbstractData::Database(database) => database.timestamp_ms,
            AbstractData::Album(album) => album.created_time as i64,
        }
    }
    pub fn hash(self: &Self) -> ArrayString<64> {
        match self {
            AbstractData::Database(database) => database.hash,
            AbstractData::Album(album) => album.id,
        }
    }
    pub fn width(self: &Self) -> u32 {
        match self {
            AbstractData::Database(database) => database.width,
            AbstractData::Album(_) => 300,
        }
    }
    pub fn height(self: &Self) -> u32 {
        match self {
            AbstractData::Database(database) => database.height,
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

    pub fn generate_token(&self, timestamp: u128) -> String {
        match self {
            AbstractData::Database(db) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(db.hash, timestamp, false).encode()
            }
            AbstractData::Album(album) => {
                use crate::router::claims::claims_hash::ClaimsHash;
                ClaimsHash::new(album.id, timestamp, false).encode()
            }
        }
    }

    pub fn with_tag(self, timestamp: u128) -> AbstractDataWithTag {
        let tag = self.tag();
        let token = self.generate_token(timestamp);
        AbstractDataWithTag {
            data: self,
            tag,
            token,
        }
    }
}

impl From<Database> for AbstractData {
    fn from(database: Database) -> Self {
        AbstractData::Database(database)
    }
}

impl From<Album> for AbstractData {
    fn from(album: Album) -> Self {
        AbstractData::Album(album)
    }
}
