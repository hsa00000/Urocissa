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
    pub fn _get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        let sql = r#"
            SELECT 
                object.id, object.obj_type, object.created_time, object.pending, object.thumbhash,
                meta_album.title, meta_album.start_time, meta_album.end_time, meta_album.last_modified_time, meta_album.cover, 
                meta_album.user_defined_metadata, meta_album.item_count, meta_album.item_size
            FROM object
            INNER JOIN meta_album ON object.id = meta_album.id
            WHERE object.id = ?
        "#;
        conn.query_row(sql, [id], Self::from_row)
    }

    /// 讀取所有相簿 (JOIN 查詢)
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        // 移除了 meta_album.tag
        let sql = r#"
            SELECT 
                object.id, object.obj_type, object.created_time, object.pending, object.thumbhash,
                meta_album.title, meta_album.start_time, meta_album.end_time, meta_album.last_modified_time, meta_album.cover, 
                meta_album.user_defined_metadata, meta_album.item_count, meta_album.item_size
            FROM object
            INNER JOIN meta_album ON object.id = meta_album.id
            WHERE object.obj_type = 'album'
        "#;

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| Self::from_row(row))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(AlbumCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: AlbumMetadataSchema::from_row(row)?,
        })
    }
}
