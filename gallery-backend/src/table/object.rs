use arrayvec::ArrayString;
use redb::TableDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use crate::public::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};

// Key: ID, Value: Serialized ObjectSchema
pub const OBJECT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("object");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
#[serde(rename_all = "camelCase")]
pub enum ObjectType {
    Image,
    Video,
    Album,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Image => write!(f, "image"),
            ObjectType::Video => write!(f, "video"),
            ObjectType::Album => write!(f, "album"),
        }
    }
}

impl FromStr for ObjectType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image" => Ok(ObjectType::Image),
            "video" => Ok(ObjectType::Video),
            "album" => Ok(ObjectType::Album),
            _ => Err(format!("Invalid ObjectType: {}", s)),
        }
    }
}

impl ObjectType {
    pub fn from_ext(ext: impl AsRef<str>) -> Option<Self> {
        let ext = ext.as_ref();
        if VALID_IMAGE_EXTENSIONS.contains(&ext) {
            Some(ObjectType::Image)
        } else if VALID_VIDEO_EXTENSIONS.contains(&ext) {
            Some(ObjectType::Video)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSchema {
    pub id: ArrayString<64>,
    pub obj_type: ObjectType,
    pub created_time: i64,
    pub pending: bool,
    pub thumbhash: Option<Vec<u8>>,
    pub description: Option<String>,
    pub tags: HashSet<String>,
}

impl ObjectSchema {
    pub fn new(id: ArrayString<64>, obj_type: ObjectType) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        Self {
            id,
            obj_type,
            created_time: timestamp,
            pending: false,
            thumbhash: None,
            description: None,
            tags: HashSet::new(),
        }
    }
}
