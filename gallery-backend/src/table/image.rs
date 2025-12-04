use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::table::meta_image::ImageMetadataSchema;
use crate::table::object::ObjectSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: ImageMetadataSchema,
    #[serde(default)]
    pub albums: HashSet<ArrayString<64>>,
}

impl ImageCombined {
    /// 根據 Hash (ID) 讀取單一圖片資料（包含所屬相簿）
    pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        // 1. 讀取本體資料
        let sql = r#"
            SELECT object.*, meta_image.* FROM object
            INNER JOIN meta_image ON object.id = meta_image.id
            WHERE object.id = ?
        "#;

        let mut image = conn.query_row(sql, [id], Self::from_row)?;

        // 2. 讀取關聯相簿 (避免 JOIN 造成多行處理的複雜度)
        let sql_albums = "SELECT album_id FROM album_database WHERE hash = ?";
        let mut stmt = conn.prepare(sql_albums)?;
        let rows = stmt.query_map([id], |row| row.get::<_, String>(0))?;

        for album_id in rows {
            if let Ok(id_str) = album_id {
                if let Ok(as_str) = ArrayString::from(&id_str) {
                    image.albums.insert(as_str);
                }
            }
        }

        Ok(image)
    }

    /// 讀取所有圖片資料（高效能批次填入相簿關聯）
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        // 1. 讀取所有圖片本體
        let sql = r#"
            SELECT object.*, meta_image.* FROM object
            INNER JOIN meta_image ON object.id = meta_image.id
            WHERE object.obj_type = 'image'
        "#;
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], Self::from_row)?;

        let mut images: Vec<Self> = rows.collect::<Result<_, _>>()?;

        // 如果沒有圖片，直接回傳空陣列，省去查關聯的開銷
        if images.is_empty() {
            return Ok(images);
        }

        // 2. 批次讀取所有「圖片」類型的相簿關聯
        let sql_relations = r#"
            SELECT ad.hash, ad.album_id 
            FROM album_database ad
            INNER JOIN object o ON ad.hash = o.id
            WHERE o.obj_type = 'image'
        "#;

        let mut stmt_rel = conn.prepare(sql_relations)?;
        let rel_rows = stmt_rel.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        // 3. 建立關聯對照表 (Hash -> Set<AlbumId>)
        let mut relation_map: HashMap<ArrayString<64>, HashSet<ArrayString<64>>> = HashMap::new();

        for rel in rel_rows {
            let (hash, album_id) = rel?;
            if let (Ok(hash_as), Ok(album_as)) =
                (ArrayString::from(&hash), ArrayString::from(&album_id))
            {
                relation_map.entry(hash_as).or_default().insert(album_as);
            }
        }

        // 4. 將相簿資料填回圖片 Struct
        for image in &mut images {
            if let Some(albums) = relation_map.remove(&image.object.id) {
                image.albums = albums;
            }
        }

        Ok(images)
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(ImageCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: ImageMetadataSchema::from_row(row)?,
            albums: HashSet::new(), // 初始為空，由呼叫端填入
        })
    }

    pub fn imported_path(&self) -> PathBuf {
        PathBuf::from(self.imported_path_string())
    }

    pub fn imported_path_string(&self) -> String {
        format!(
            "./object/imported/{}/{}.{}",
            &self.object.id[0..2],
            self.object.id,
            self.metadata.ext
        )
    }
}
