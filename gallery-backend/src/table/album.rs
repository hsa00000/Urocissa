use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use arrayvec::ArrayString;

use crate::table::meta_album::AlbumMetadataSchema;
use crate::table::object::ObjectSchema;

/// 這是給 API 回傳用的組合結構，透過 serde(flatten) 保持 JSON 格式與舊版相容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: AlbumMetadataSchema,
    #[serde(default)]
    pub tags: HashSet<String>,
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
        // 1. 讀取相簿本體
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
        let mut albums: Vec<Self> = rows.collect::<rusqlite::Result<_>>()?;

        if albums.is_empty() {
            return Ok(albums);
        }

        // 2. 批次讀取所有「相簿」類型的標籤關聯
        let sql_tag_relations = r#"
            SELECT td.hash, td.tag
            FROM tag_database td
            INNER JOIN object o ON td.hash = o.id
            WHERE o.obj_type = 'album'
        "#;

        let mut stmt_tag_rel = conn.prepare(sql_tag_relations)?;
        let tag_rel_rows = stmt_tag_rel.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut tag_map: HashMap<ArrayString<64>, HashSet<String>> = HashMap::new();

        for rel in tag_rel_rows {
            let (hash, tag) = rel?;
            if let Ok(hash_as) = ArrayString::from(&hash) {
                tag_map.entry(hash_as).or_default().insert(tag);
            }
        }

        // 3. 將資料填回相簿 Struct
        for album in &mut albums {
            if let Some(tags) = tag_map.remove(&album.object.id) {
                album.tags = tags;
            }
        }

        Ok(albums)
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(AlbumCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: AlbumMetadataSchema::from_row(row)?,
            tags: HashSet::new(),
        })
    }
}
