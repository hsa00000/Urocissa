use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadataSchema {
    pub id: ArrayString<64>, // FK to object.id
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub duration: f64, // 影片時長 (秒)
}

impl VideoMetadataSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS meta_video (
                id TEXT PRIMARY KEY,
                size INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                ext TEXT NOT NULL,
                duration REAL DEFAULT 0.0,
                FOREIGN KEY(id) REFERENCES object(id) ON DELETE CASCADE
            );
        "#;
        conn.execute(sql, [])?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id_str: String = row.get("id")?;
        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            size: row.get("size")?,
            width: row.get("width")?,
            height: row.get("height")?,
            ext: row.get("ext")?,
            duration: row.get("duration")?,
        })
    }

    pub fn new(id: ArrayString<64>, size: u64, width: u32, height: u32, ext: String) -> Self {
        Self {
            id,
            size,
            width,
            height,
            ext,
            duration: 0.0,
        }
    }
}