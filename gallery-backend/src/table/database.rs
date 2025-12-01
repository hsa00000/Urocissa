use arrayvec::ArrayString;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// DatabaseSchema: 資料庫層的 schema，用於從 SQLite 讀取/寫入
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
    pub album: HashSet<ArrayString<64>>,
    pub ext_type: String,
    pub pending: bool,
    pub timestamp_ms: i64,
}

impl DatabaseSchema {
    pub fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        let sql_create_main_table = r#"
            CREATE TABLE IF NOT EXISTS database (
                hash TEXT PRIMARY KEY,
                size INTEGER CHECK(size > 0),
                width INTEGER CHECK(width > 0),
                height INTEGER CHECK(height > 0),
                thumbhash BLOB,
                phash BLOB,
                ext TEXT NOT NULL,
                album TEXT,
                ext_type TEXT CHECK(ext_type IN ('image', 'video')),
                pending INTEGER,
                timestamp_ms INTEGER CHECK(timestamp_ms > 0)
            );
        "#;
        conn.execute(sql_create_main_table, [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_database_timestamp ON database(timestamp_ms);",
            [],
        )?;

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

        let album_str: String = row.get("album")?;
        let album_vec: Vec<String> = serde_json::from_str(&album_str).unwrap_or_default();
        let album: HashSet<ArrayString<64>> = album_vec
            .into_iter()
            .filter_map(|s| ArrayString::from(&s).ok())
            .collect();

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
            album,
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

    pub fn compressed_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.compressed_path_string())
    }

    pub fn thumbnail_path(&self) -> String {
        format!("./object/compressed/{}/{}.jpg", &self.hash[0..2], self.hash)
    }

    pub fn compressed_path_parent(&self) -> std::path::PathBuf {
        self.compressed_path()
            .parent()
            .expect("Path::new(&output_file_path_string).parent() fail")
            .to_path_buf()
    }

    pub fn generate_random_data() -> Self {
        use crate::workflow::operations::hash::generate_random_hash;
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
            album: HashSet::new(),
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
            album: HashSet::new(),
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
