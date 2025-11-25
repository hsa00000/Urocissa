use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Deserialize, Default, Serialize, PartialEq, Eq)]
pub struct DatabaseSchema {
    pub hash: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub thumbhash: Vec<u8>,
    pub phash: Vec<u8>,
    pub ext: String,
    pub album: HashSet<ArrayString<64>>,
    pub ext_type: String,
    pub pending: bool,
    pub timestamp_ms: i64,
}

impl DatabaseSchema {
    pub fn create_database_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql_create_main_table = r#"
            CREATE TABLE IF NOT EXISTS database (
                hash TEXT PRIMARY KEY,
                size INTEGER,
                width INTEGER,
                height INTEGER,
                thumbhash BLOB,
                phash BLOB,
                ext TEXT,
                exif_vec TEXT,
                album TEXT,
                ext_type TEXT,
                pending INTEGER,
                timestamp_ms INTEGER
            );
        "#;
        conn.execute(sql_create_main_table, [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_database_timestamp ON database(timestamp_ms);",
            [],
        )?;

        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let hash: String = row.get("hash")?;
        let size: u64 = row.get("size")?;
        let width: u32 = row.get("width")?;
        let height: u32 = row.get("height")?;
        let thumbhash: Vec<u8> = row.get("thumbhash")?;
        let phash: Vec<u8> = row.get("phash")?;
        let ext: String = row.get("ext")?;

        let album_str: String = row.get("album")?;
        let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap_or_default();
        let album: HashSet<ArrayString<64>> = album_vec
            .into_iter()
            .filter_map(|s| ArrayString::from(&s).ok())
            .collect();

        let ext_type: String = row.get("ext_type")?;
        let pending: bool = row.get::<_, i32>("pending")? != 0;

        let timestamp_ms: i64 = row.get("timestamp_ms").unwrap_or(0);

        Ok(DatabaseSchema {
            hash: ArrayString::from(&hash).unwrap(),
            size,
            width,
            height,
            thumbhash,
            phash,
            ext,
            album,
            ext_type,
            pending,
            timestamp_ms,
        })
    }
}
