use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use crate::table::meta_image::ImageMetadataSchema;
use crate::table::object::ObjectSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: ImageMetadataSchema,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
}

impl ImageCombined {
    /// 根據 Hash (ID) 讀取單一圖片資料（包含所屬相簿、標籤與 EXIF）
    pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        // 1. 讀取本體資料
        let sql = r#"
            SELECT object.*, meta_image.* FROM object
            INNER JOIN meta_image ON object.id = meta_image.id
            WHERE object.id = ?
        "#;

        let mut image = conn.query_row(sql, [id], Self::from_row)?;

        // 2. 讀取關聯相簿
        let sql_albums = "SELECT album_id FROM album_database WHERE hash = ?";
        let mut stmt_albums = conn.prepare(sql_albums)?;
        let album_rows = stmt_albums.query_map([id], |row| row.get::<_, String>(0))?;

        for album_id in album_rows {
            if let Ok(id_str) = album_id {
                if let Ok(as_str) = ArrayString::from(&id_str) {
                    image.albums.insert(as_str);
                }
            }
        }

        // 3. 讀取關聯標籤 (填入 object.tags)
        let sql_tags = "SELECT tag FROM tag_database WHERE hash = ?";
        let mut stmt_tags = conn.prepare(sql_tags)?;
        let tag_rows = stmt_tags.query_map([id], |row| row.get::<_, String>(0))?;

        for tag in tag_rows {
            if let Ok(t) = tag {
                image.object.tags.insert(t);
            }
        }

        // 4. 讀取 EXIF
        let sql_exif = "SELECT tag, value FROM database_exif WHERE hash = ?";
        let mut stmt_exif = conn.prepare(sql_exif)?;
        let exif_rows = stmt_exif.query_map([id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in exif_rows {
            if let Ok((k, v)) = row {
                image.exif_vec.insert(k, v);
            }
        }

        Ok(image)
    }

    /// 讀取所有圖片資料（高效能批次填入相簿、標籤與 EXIF 關聯）
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

        if images.is_empty() {
            return Ok(images);
        }

        // 2. 批次讀取所有「圖片」類型的相簿關聯
        let sql_album_relations = r#"
            SELECT ad.hash, ad.album_id 
            FROM album_database ad
            INNER JOIN object o ON ad.hash = o.id
            WHERE o.obj_type = 'image'
        "#;

        let mut stmt_album_rel = conn.prepare(sql_album_relations)?;
        let album_rel_rows = stmt_album_rel.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut album_map: HashMap<ArrayString<64>, HashSet<ArrayString<64>>> = HashMap::new();

        for rel in album_rel_rows {
            let (hash, album_id) = rel?;
            if let (Ok(hash_as), Ok(album_as)) =
                (ArrayString::from(&hash), ArrayString::from(&album_id))
            {
                album_map.entry(hash_as).or_default().insert(album_as);
            }
        }

        // 3. 批次讀取所有「圖片」類型的標籤關聯
        let sql_tag_relations = r#"
            SELECT td.hash, td.tag
            FROM tag_database td
            INNER JOIN object o ON td.hash = o.id
            WHERE o.obj_type = 'image'
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

        // 4. 批次讀取所有「圖片」類型的 EXIF
        let sql_exif_relations = r#"
            SELECT de.hash, de.tag, de.value
            FROM database_exif de
            INNER JOIN object o ON de.hash = o.id
            WHERE o.obj_type = 'image'
        "#;

        let mut stmt_exif_rel = conn.prepare(sql_exif_relations)?;
        let exif_rel_rows = stmt_exif_rel.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut exif_map: HashMap<ArrayString<64>, BTreeMap<String, String>> = HashMap::new();

        for rel in exif_rel_rows {
            let (hash, k, v) = rel?;
            if let Ok(hash_as) = ArrayString::from(&hash) {
                exif_map.entry(hash_as).or_default().insert(k, v);
            }
        }

        // 5. 將資料填回圖片 Struct
        for image in &mut images {
            if let Some(albums) = album_map.remove(&image.object.id) {
                image.albums = albums;
            }
            if let Some(tags) = tag_map.remove(&image.object.id) {
                image.object.tags = tags;
            }
            if let Some(exif) = exif_map.remove(&image.object.id) {
                image.exif_vec = exif;
            }
        }

        Ok(images)
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(ImageCombined {
            object: ObjectSchema::from_row(row)?,
            metadata: ImageMetadataSchema::from_row(row)?,
            albums: HashSet::new(),
            exif_vec: BTreeMap::new(),
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
