use crate::{
    public::constant::VALID_IMAGE_EXTENSIONS,
    public::structure::database_struct::{database::definition::DatabaseWithTag, file_modify::FileModify},
};
use anyhow::Context;
use anyhow::Result;
use arrayvec::ArrayString;
use std::{
    collections::{BTreeMap, HashSet},
    fs::metadata,
    path::Path,
    time::UNIX_EPOCH,
};

impl DatabaseWithTag {
    pub fn new(path: &Path, hash: ArrayString<64>) -> Result<Self> {
        let ext = path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("File has no extension: {:?}", path))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Extension is not valid UTF-8: {:?}", path))?
            .to_ascii_lowercase();

        let md = metadata(path).with_context(|| format!("Failed to read metadata: {:?}", path))?;
        let size = md.len();

        let modified_millis = md
            .modified()?
            .duration_since(UNIX_EPOCH)
            .with_context(|| format!("Modification time is before UNIX_EPOCH: {:?}", path))?
            .as_millis();

        let file_modify = FileModify::new(path, modified_millis);

        Ok(Self {
            hash,
            size,
            width: 0,
            height: 0,
            thumbhash: Vec::new(),
            phash: Vec::new(),
            ext_type: Self::determine_type(&ext),
            ext,
            exif_vec: BTreeMap::new(),
            tag: HashSet::new(),
            album: HashSet::new(),
            alias: vec![file_modify],
            pending: false,
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
