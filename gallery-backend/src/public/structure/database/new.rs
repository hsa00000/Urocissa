use crate::public::constant::VALID_IMAGE_EXTENSIONS;
use crate::public::structure::database::definition::DatabaseSchema;
use anyhow::Context;
use anyhow::Result;
use arrayvec::ArrayString;
use std::{
    collections::{BTreeMap, HashSet},
    fs::metadata,
    path::Path,
    time::UNIX_EPOCH,
};

impl DatabaseSchema {
    pub fn new(path: &Path, hash: ArrayString<64>) -> Result<Self> {
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
        if VALID_IMAGE_EXTENSIONS.contains(&ext) {
            "image"
        } else {
            "video"
        }
        .into()
    }
}
