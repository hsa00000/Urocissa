use crate::public::structure::abstract_data::AbstractData;
use arrayvec::ArrayString;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct IndexTask {
    pub imported_path: PathBuf,
    pub data: AbstractData,
    pub width: u32,
    pub height: u32,
    pub size: u64,
    pub thumbhash: Vec<u8>,
    pub phash: Vec<u8>,
    pub exif_vec: BTreeMap<String, String>,
    pub ext_type: String,
}

impl IndexTask {
    pub fn new(imported_path: PathBuf, data: AbstractData) -> Self {
        let (width, height, size, thumbhash, phash, ext_type) = match &data {
            AbstractData::Image(i) => (
                i.metadata.width,
                i.metadata.height,
                i.metadata.size,
                i.object.thumbhash.clone().unwrap_or_default(),
                i.metadata.phash.clone().unwrap_or_default(),
                "image".to_string(),
            ),
            AbstractData::Video(v) => (
                v.metadata.width,
                v.metadata.height,
                v.metadata.size,
                v.object.thumbhash.clone().unwrap_or_default(),
                Vec::new(),
                "video".to_string(),
            ),
            _ => (0, 0, 0, Vec::new(), Vec::new(), "unknown".to_string()),
        };

        Self {
            imported_path,
            data,
            width,
            height,
            size,
            thumbhash,
            phash,
            exif_vec: BTreeMap::new(),
            ext_type,
        }
    }

    // Helper methods...
    pub fn imported_path(&self) -> PathBuf {
        self.imported_path.clone()
    }

    pub fn compressed_path(&self) -> PathBuf {
        // Logic to generate compressed path based on hash
        let hash = self.hash();
        if self.ext_type == "image" {
             PathBuf::from(format!("./object/compressed/{}/{}.jpg", &hash[0..2], hash))
        } else {
             PathBuf::from(format!("./object/compressed/{}/{}.mp4", &hash[0..2], hash))
        }
    }
    
    pub fn thumbnail_path(&self) -> String {
         let hash = self.hash();
         format!("./object/compressed/{}/{}.jpg", &hash[0..2], hash)
    }

    pub fn hash(&self) -> ArrayString<64> {
        match &self.data {
            AbstractData::Image(i) => i.object.id,
            AbstractData::Video(v) => v.object.id,
            AbstractData::Album(a) => a.object.id,
        }
    }
}

// Implement From<IndexTask> for AbstractData (or update logic)
impl From<IndexTask> for AbstractData {
    fn from(task: IndexTask) -> Self {
        match task.data {
            AbstractData::Image(mut i) => {
                i.metadata.width = task.width;
                i.metadata.height = task.height;
                i.metadata.size = task.size;
                i.object.thumbhash = Some(task.thumbhash);
                i.metadata.phash = Some(task.phash);
                AbstractData::Image(i)
            }
            AbstractData::Video(mut v) => {
                v.metadata.width = task.width;
                v.metadata.height = task.height;
                v.metadata.size = task.size;
                v.object.thumbhash = Some(task.thumbhash);
                AbstractData::Video(v)
            }
            other => other,
        }
    }
}
