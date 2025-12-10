use crate::models::entity::abstract_data::AbstractData;
use crate::database::schema::object::ObjectType;
use crate::utils::compressed_path;
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
    pub fn new(imported_path: impl Into<PathBuf>, data: AbstractData) -> Self {
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
            imported_path: imported_path.into(),
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
    pub fn compressed_path(&self) -> PathBuf {
        let obj_type = match self.ext_type.as_str() {
            "image" => ObjectType::Image,
            "video" => ObjectType::Video,
            _ => panic!("Unknown ext_type: {}", self.ext_type),
        };
        compressed_path(self.hash(), obj_type)
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
