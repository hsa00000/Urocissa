use std::collections::{HashMap, HashSet};
use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

use crate::table::object::ObjectSchema;

/// AlbumMetadataSchema: 相簿專用屬性
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumMetadataSchema {
    pub id: ArrayString<64>, // FK to object.id
    pub title: Option<String>,
    pub start_time: Option<i64>, // 使用 i64 以保持與 DB 一致
    pub end_time: Option<i64>,
    pub last_modified_time: i64,
    pub cover: Option<ArrayString<64>>,
    // 這些 JSON 欄位保留在這裡
    pub user_defined_metadata: HashMap<String, Vec<String>>,
    pub tag: HashSet<String>,
    pub item_count: usize,
    pub item_size: u64,
}

impl AlbumMetadataSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS meta_album (
                id TEXT PRIMARY KEY,
                title TEXT,
                start_time INTEGER,
                end_time INTEGER,
                last_modified_time INTEGER,
                cover TEXT,
                user_defined_metadata TEXT,
                tag TEXT,
                item_count INTEGER DEFAULT 0,
                item_size INTEGER DEFAULT 0,
                FOREIGN KEY(id) REFERENCES object(id) ON DELETE CASCADE
            );
        "#;
        conn.execute(sql, [])?;
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id_str: String = row.get("id")?;
        let title: Option<String> = row.get("title")?;
        let start_time: Option<i64> = row.get("start_time")?;
        let end_time: Option<i64> = row.get("end_time")?;
        let last_modified_time: i64 = row.get("last_modified_time")?;
        
        let cover_str: Option<String> = row.get("cover")?;
        let cover: Option<ArrayString<64>> = cover_str.and_then(|s| ArrayString::from(&s).ok());
        
        let user_defined_metadata_str: String = row.get("user_defined_metadata")?;
        let user_defined_metadata: HashMap<String, Vec<String>> =
            serde_json::from_str(&user_defined_metadata_str).unwrap_or_default();
            
        let tag_str: String = row.get("tag")?;
        let tag: HashSet<String> = serde_json::from_str(&tag_str).unwrap_or_default();
        
        let item_count: usize = row.get::<_, i64>("item_count")? as usize;
        let item_size: u64 = row.get("item_size")?;

        Ok(Self {
            id: ArrayString::from(&id_str).unwrap(),
            title,
            start_time,
            end_time,
            last_modified_time,
            cover,
            user_defined_metadata,
            tag,
            item_count,
            item_size,
        })
    }

    pub fn new(id: ArrayString<64>, title: Option<String>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        Self {
            id,
            title,
            start_time: None,
            end_time: None,
            last_modified_time: timestamp,
            cover: None,
            user_defined_metadata: HashMap::new(),
            tag: HashSet::new(),
            item_count: 0,
            item_size: 0,
        }
    }
}

/// 這是給 API 回傳用的組合結構，透過 serde(flatten) 保持 JSON 格式與舊版相容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: AlbumMetadataSchema,
}

impl AlbumCombined {
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
}
