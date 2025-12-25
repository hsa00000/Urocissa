use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use crate::public::structure::common::FileModify;

/// Video-specific metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub duration: f64,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
    pub alias: Vec<FileModify>,
}

impl VideoMetadata {
    pub fn new(id: ArrayString<64>, size: u64, width: u32, height: u32, ext: String) -> Self {
        Self {
            id,
            size,
            width,
            height,
            ext,
            duration: 0.0,
            albums: HashSet::new(),
            exif_vec: BTreeMap::new(),
            alias: Vec::new(),
        }
    }
}
