use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use crate::public::structure::common::FileModify;

/// Image-specific metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub phash: Option<Vec<u8>>,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
    pub alias: Vec<FileModify>,
}

impl ImageMetadata {
    pub fn new(id: ArrayString<64>, size: u64, width: u32, height: u32, ext: String) -> Self {
        Self {
            id,
            size,
            width,
            height,
            ext,
            phash: None,
            albums: HashSet::new(),
            exif_vec: BTreeMap::new(),
            alias: Vec::new(),
        }
    }
}
