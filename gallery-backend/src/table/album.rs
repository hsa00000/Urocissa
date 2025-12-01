use std::collections::{HashMap, HashSet};

use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

/// AlbumSchema: 資料庫層的 Album schema，用於從 SQLite 讀取/寫入
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSchema {
    pub id: ArrayString<64>,
    pub title: Option<String>,
    pub created_time: u128,
    pub start_time: Option<u128>,
    pub end_time: Option<u128>,
    pub last_modified_time: u128,
    pub cover: Option<ArrayString<64>>,
    pub thumbhash: Option<Vec<u8>>,
    pub user_defined_metadata: HashMap<String, Vec<String>>,
    pub tag: HashSet<String>,
    pub item_count: usize,
    pub item_size: u64,
    pub pending: bool,
}

impl AlbumSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
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
                tag TEXT,
                item_count INTEGER,
                item_size INTEGER,
                pending INTEGER
            );
        "#;
        conn.execute(sql, [])?;
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
        let tag_str: String = row.get("tag")?;
        let tag: HashSet<String> = serde_json::from_str(&tag_str).unwrap_or_default();
        let item_count: usize = row.get::<_, i64>("item_count")? as usize;
        let item_size: u64 = row.get("item_size")?;
        let pending: bool = row.get::<_, i32>("pending")? != 0;
        Ok(AlbumSchema {
            id: ArrayString::from(&id).unwrap(),
            title,
            created_time,
            start_time,
            end_time,
            last_modified_time,
            cover,
            thumbhash,
            user_defined_metadata,
            tag,
            item_count,
            item_size,
            pending,
        })
    }

    pub fn new(id: ArrayString<64>, title: Option<String>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Self {
            id,
            title,
            created_time: timestamp,
            cover: None,
            thumbhash: None,
            user_defined_metadata: HashMap::new(),
            tag: HashSet::new(),
            start_time: None,
            end_time: None,
            last_modified_time: timestamp,
            item_count: 0,
            item_size: 0,
            pending: false,
        }
    }
}
