use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
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

/// Common object schema shared between Image, Video, and Album
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct ObjectSchema {
    pub id: ArrayString<64>,
    pub obj_type: ObjectType,
    pub pending: bool,
    pub thumbhash: Option<Vec<u8>>,
    pub description: Option<String>,
    pub tags: HashSet<String>,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub is_trashed: bool,
}

impl ObjectSchema {
    pub fn new(id: ArrayString<64>, obj_type: ObjectType) -> Self {
        Self {
            id,
            obj_type,
            pending: false,
            thumbhash: None,
            description: None,
            tags: HashSet::new(),
            is_favorite: false,
            is_archived: false,
            is_trashed: false,
        }
    }
}
