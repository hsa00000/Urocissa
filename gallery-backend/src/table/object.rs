use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

// [新增]: 引入常量定義
use crate::public::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObjectType {
    Image,
    Video,
    Album,
}

impl ObjectType {
    /// 根據副檔名判斷類型
    pub fn from_ext(ext: &str) -> Option<Self> {
        if VALID_IMAGE_EXTENSIONS.contains(&ext) {
            Some(ObjectType::Image)
        } else if VALID_VIDEO_EXTENSIONS.contains(&ext) {
            Some(ObjectType::Video)
        } else {
            None
        }
    }
}

/// ObjectSchema: 系統中所有實體的共同基類
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSchema {
    pub id: ArrayString<64>,
    pub obj_type: String, // "image", "video", "album"
    pub created_time: i64,
    pub pending: bool,
    pub thumbhash: Option<Vec<u8>>,
}

impl ObjectSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS object (
                id TEXT PRIMARY KEY,
                obj_type TEXT NOT NULL CHECK(obj_type IN ('image', 'video', 'album')),
                created_time INTEGER NOT NULL,
                pending INTEGER DEFAULT 0,
                thumbhash BLOB
            );
            CREATE INDEX IF NOT EXISTS idx_object_created_time ON object(created_time);
            CREATE INDEX IF NOT EXISTS idx_object_type ON object(obj_type);
        "#;
        conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id_str: String = row.get("id")?;
        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            obj_type: row.get("obj_type")?,
            created_time: row.get("created_time")?,
            pending: row.get("pending")?,
            thumbhash: row.get("thumbhash")?,
        })
    }

    pub fn new(id: ArrayString<64>, obj_type: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        Self {
            id,
            obj_type: obj_type.to_string(),
            created_time: timestamp,
            pending: false,
            thumbhash: None,
        }
    }
}
