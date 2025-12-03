use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};
use std::path::Path;

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