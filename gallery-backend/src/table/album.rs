use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

use crate::table::object::ObjectSchema;
use crate::table::meta_album::AlbumMetadataSchema;

/// 這是給 API 回傳用的組合結構，透過 serde(flatten) 保持 JSON 格式與舊版相容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: AlbumMetadataSchema,
}

impl AlbumCombined {
    /// 根據 Hash (ID) 讀取單一相簿資料
    pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        let sql = r#"
            SELECT o.*, m.* FROM object o
            INNER JOIN meta_album m ON o.id = m.id
            WHERE o.id = ?
        "#;
        conn.query_row(sql, [id], Self::from_row)
    }

    /// 讀取所有相簿 (JOIN 查詢)
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        let sql = r#"
            SELECT 
                o.id, o.obj_type, o.created_time, o.pending, o.thumbhash,
                m.title, m.start_time, m.end_time, m.last_modified_time, m.cover, 
                m.user_defined_metadata, m.tag, m.item_count, m.item_size
            FROM object o
            INNER JOIN meta_album m ON o.id = m.id
            WHERE o.obj_type = 'album'
        "#;

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| Self::from_joined_row(row))?;

        let mut albums = Vec::new();
        for album in rows {
            albums.push(album?);
        }
        Ok(albums)
    }

    /// 從 JOIN 後的 Row 解析資料
    fn from_joined_row(row: &Row) -> rusqlite::Result<Self> {
        let object = ObjectSchema::from_row(row)?;
        let metadata = AlbumMetadataSchema::from_row(row)?;

        Ok(Self { object, metadata })
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(AlbumCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: AlbumMetadataSchema::from_row(row)?,
        })
    }
}
