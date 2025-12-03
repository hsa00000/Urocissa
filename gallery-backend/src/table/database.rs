use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::table::object::ObjectSchema;
use crate::table::meta_image::ImageMetadataSchema;
use crate::table::meta_video::VideoMetadataSchema;

// 統一的媒體列舉，用於 load_from_db 回傳
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MediaCombined {
    Image(ImageCombined),
    Video(VideoCombined),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: ImageMetadataSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: VideoMetadataSchema,
}

impl MediaCombined {
    /// 根據 Hash (ID) 讀取單一媒體資料
    pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Self> {
        // 1. 先查詢 object 表確認類型
        let type_sql = "SELECT obj_type FROM object WHERE id = ?";
        let obj_type: String = conn.query_row(type_sql, [id], |row| row.get(0))?;

        // 2. 根據類型執行對應的 JOIN
        match obj_type.as_str() {
            "image" => {
                let sql = r#"
                    SELECT o.*, m.* FROM object o
                    INNER JOIN meta_image m ON o.id = m.id
                    WHERE o.id = ?
                "#;
                conn.query_row(sql, [id], |row| {
                    Ok(MediaCombined::Image(ImageCombined {
                        object: ObjectSchema::from_row(row)?,
                        metadata: ImageMetadataSchema::from_row(row)?,
                    }))
                })
            }
            "video" => {
                let sql = r#"
                    SELECT o.*, m.* FROM object o
                    INNER JOIN meta_video m ON o.id = m.id
                    WHERE o.id = ?
                "#;
                conn.query_row(sql, [id], |row| {
                    Ok(MediaCombined::Video(VideoCombined {
                        object: ObjectSchema::from_row(row)?,
                        metadata: VideoMetadataSchema::from_row(row)?,
                    }))
                })
            }
            _ => Err(rusqlite::Error::QueryReturnedNoRows), // 或其他錯誤
        }
    }

    /// 讀取所有媒體資料
    pub fn get_all(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        let mut result = Vec::new();

        // 讀取所有 image
        let image_sql = r#"
            SELECT o.*, m.* FROM object o
            INNER JOIN meta_image m ON o.id = m.id
            WHERE o.obj_type = 'image'
        "#;
        let mut stmt = conn.prepare(image_sql)?;
        let image_rows = stmt.query_map([], |row| {
            Ok(MediaCombined::Image(ImageCombined {
                object: ObjectSchema::from_row(row)?,
                metadata: ImageMetadataSchema::from_row(row)?,
            }))
        })?;
        for row in image_rows {
            result.push(row?);
        }

        // 讀取所有 video
        let video_sql = r#"
            SELECT o.*, m.* FROM object o
            INNER JOIN meta_video m ON o.id = m.id
            WHERE o.obj_type = 'video'
        "#;
        let mut stmt = conn.prepare(video_sql)?;
        let video_rows = stmt.query_map([], |row| {
            Ok(MediaCombined::Video(VideoCombined {
                object: ObjectSchema::from_row(row)?,
                metadata: VideoMetadataSchema::from_row(row)?,
            }))
        })?;
        for row in video_rows {
            result.push(row?);
        }

        Ok(result)
    }
}

// 保留舊的 DatabaseSchema 用於向後兼容或遷移
/// DatabaseSchema: 舊的 schema，保留用於遷移
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSchema {
    pub hash: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub thumbhash: Vec<u8>,
    pub phash: Vec<u8>,
    pub ext: String,
    pub ext_type: String,
    pub pending: bool,
    pub timestamp_ms: i64,
}

impl DatabaseSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        // 舊表已棄用，不再創建
        Ok(())
    }

    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let hash: String = row.get("hash")?;
        let size: u64 = row.get("size")?;
        let width: u32 = row.get("width")?;
        let height: u32 = row.get("height")?;
        let thumbhash: Vec<u8> = row.get("thumbhash")?;
        let phash: Vec<u8> = row.get("phash")?;
        let ext: String = row.get("ext")?;
        let ext_type: String = row.get("ext_type")?;
        let pending: bool = row.get::<_, i32>("pending")? != 0;
        let timestamp_ms: i64 = row.get("timestamp_ms").unwrap_or(0);

        Ok(DatabaseSchema {
            hash: ArrayString::from(&hash).unwrap(),
            size,
            width,
            height,
            thumbhash,
            phash,
            ext,
            ext_type,
            pending,
            timestamp_ms,
        })
    }

    pub fn imported_path_string(&self) -> String {
        format!(
            "./object/imported/{}/{}.{}",
            &self.hash[0..2],
            self.hash,
            self.ext
        )
    }

    pub fn imported_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.imported_path_string())
    }

    pub fn compressed_path_string(&self) -> String {
        if self.ext_type == "image" {
            format!("./object/compressed/{}/{}.jpg", &self.hash[0..2], self.hash)
        } else {
            format!("./object/compressed/{}/{}.mp4", &self.hash[0..2], self.hash)
        }
    }

    pub fn generate_random_data() -> Self {
        use crate::workflow::processors::file::generate_random_hash;
        use rand::Rng;

        let hash = generate_random_hash();
        let width = rand::rng().random_range(300..=600);
        let height = rand::rng().random_range(300..=600);

        Self {
            size: 0,
            hash,
            width,
            height,
            thumbhash: Vec::<u8>::new(),
            phash: Vec::<u8>::new(),
            ext_type: "image".to_string(),
            ext: "jpg".to_string(),
            pending: false,
            timestamp_ms: 0,
        }
    }

    pub fn new(path: &Path, hash: ArrayString<64>) -> anyhow::Result<Self> {
        use anyhow::Context;
        use std::fs::metadata;

        let ext = path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("File has no extension: {:?}", path))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Extension is not valid UTF-8: {:?}", path))?
            .to_ascii_lowercase();

        let md = metadata(path).with_context(|| format!("Failed to read metadata: {:?}", path))?;
        let size = md.len();

        Ok(Self {
            hash,
            size,
            width: 0,
            height: 0,
            thumbhash: Vec::new(),
            phash: Vec::new(),
            ext_type: Self::determine_type(&ext),
            ext,
            pending: false,
            timestamp_ms: 0,
        })
    }

    fn determine_type(ext: &str) -> String {
        use crate::public::constant::VALID_IMAGE_EXTENSIONS;
        if VALID_IMAGE_EXTENSIONS.contains(&ext) {
            "image"
        } else {
            "video"
        }
        .into()
    }
}