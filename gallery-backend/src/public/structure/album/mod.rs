use std::collections::{HashMap, HashSet};

use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json;

pub mod edit;
pub mod new;

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub url: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: u64,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedShare {
    pub share: Share,
    pub album_id: ArrayString<64>,
    pub album_title: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: ArrayString<64>,
    pub title: Option<String>,
    pub created_time: u128,
    pub start_time: Option<u128>,
    pub end_time: Option<u128>,
    pub last_modified_time: u128,
    pub cover: Option<ArrayString<64>>,
    pub thumbhash: Option<Vec<u8>>,
    pub user_defined_metadata: HashMap<String, Vec<String>>,
    pub share_list: HashMap<ArrayString<64>, Share>,
    pub tag: HashSet<String>,
    pub width: u32,
    pub height: u32,
    pub item_count: usize,
    pub item_size: u64,
    pub pending: bool,
}

impl Album {
    pub fn create_album_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS album (
                id TEXT PRIMARY KEY,
                title TEXT,
                created_time INTEGER,
                start_time INTEGER,
                end_time INTEGER,
                last_modified_time INTEGER,
                cover TEXT,
                thumbhash BLOB,
                user_defined_metadata TEXT,
                share_list TEXT,
                tag TEXT,
                width INTEGER,
                height INTEGER,
                item_count INTEGER,
                item_size INTEGER,
                pending INTEGER
            );
        "#;
        conn.execute(sql, [])?;
        Ok(())
    }

    pub fn create_album_databases_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS album_databases (
                album_id TEXT NOT NULL,
                hash     TEXT NOT NULL,
                PRIMARY KEY (album_id, hash),
                FOREIGN KEY (album_id) REFERENCES album(id) ON DELETE CASCADE,
                FOREIGN KEY (hash)     REFERENCES database(hash) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_album_databases_album_id
                ON album_databases(album_id);

            CREATE INDEX IF NOT EXISTS idx_album_databases_hash
                ON album_databases(hash);
        "#;
        conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id: String = row.get("id")?;
        let title: Option<String> = row.get("title")?;
        let created_time: u128 = row.get::<_, i64>("created_time")? as u128;
        let start_time: Option<u128> = row.get::<_, Option<i64>>("start_time")?.map(|t| t as u128);
        let end_time: Option<u128> = row.get::<_, Option<i64>>("end_time")?.map(|t| t as u128);
        let last_modified_time: u128 = row.get::<_, i64>("last_modified_time")? as u128;
        let cover_str: Option<String> = row.get("cover")?;
        let cover: Option<ArrayString<64>> = cover_str.and_then(|s| ArrayString::from(&s).ok());
        let thumbhash: Option<Vec<u8>> = row.get("thumbhash")?;
        let user_defined_metadata_str: String = row.get("user_defined_metadata")?;
        let user_defined_metadata: HashMap<String, Vec<String>> =
            serde_json::from_str(&user_defined_metadata_str).unwrap_or_default();
        let share_list_str: String = row.get("share_list")?;
        let share_list: HashMap<ArrayString<64>, Share> =
            serde_json::from_str(&share_list_str).unwrap_or_default();
        let tag_str: String = row.get("tag")?;
        let tag: HashSet<String> = serde_json::from_str(&tag_str).unwrap_or_default();
        let width: u32 = row.get("width")?;
        let height: u32 = row.get("height")?;
        let item_count: usize = row.get::<_, i64>("item_count")? as usize;
        let item_size: u64 = row.get("item_size")?;
        let pending: bool = row.get::<_, i32>("pending")? != 0;
        Ok(Album {
            id: ArrayString::from(&id).unwrap(),
            title,
            created_time,
            start_time,
            end_time,
            last_modified_time,
            cover,
            thumbhash,
            user_defined_metadata,
            share_list,
            tag,
            width,
            height,
            item_count,
            item_size,
            pending,
        })
    }
}
