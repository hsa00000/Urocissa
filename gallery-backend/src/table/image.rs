use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    /// 根據 Hash (ID) 讀取單一圖片資料
    pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        let sql = r#"
            SELECT object.*, meta_image.* FROM object
            INNER JOIN meta_image ON object.id = meta_image.id
            WHERE object.id = ?
        "#;
        conn.query_row(sql, [id], Self::from_row)
    }

    /// 讀取所有圖片資料
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        let sql = r#"
            SELECT object.*, meta_image.* FROM object
            INNER JOIN meta_image ON object.id = meta_image.id
            WHERE object.obj_type = 'image'
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
        Ok(ImageCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: ImageMetadataSchema::from_row(row)?,
            albums: HashSet::new(),
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
