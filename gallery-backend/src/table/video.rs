use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

use crate::table::object::ObjectSchema;
use crate::table::meta_video::VideoMetadataSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: VideoMetadataSchema,
}

impl VideoCombined {
    /// 根據 Hash (ID) 讀取單一影片資料
    pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        let sql = r#"
            SELECT o.*, m.* FROM object o
            INNER JOIN meta_video m ON o.id = m.id
            WHERE o.id = ?
        "#;
        conn.query_row(sql, [id], Self::from_row)
    }

    /// 讀取所有影片資料
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        let sql = r#"
            SELECT o.*, m.* FROM object o
            INNER JOIN meta_video m ON o.id = m.id
            WHERE o.obj_type = 'video'
        "#;
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], Self::from_row)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(VideoCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: VideoMetadataSchema::from_row(row)?,
        })
    }
}