//! Frozen reader for the V5 bitcode database.
//!
//! Do not change these structs. They describe the bytes written by the last
//! bitcode-backed release and are intentionally separate from the current
//! runtime/domain structs.

#![allow(clippy::struct_excessive_bools)]

use std::collections::{BTreeMap, HashMap, HashSet};

use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use redb::{TypeName, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum LegacyAbstractData {
    Image(LegacyImageCombined),
    Video(LegacyVideoCombined),
    Album(LegacyAlbumCombined),
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyImageCombined {
    pub object: LegacyObjectSchema,
    pub metadata: LegacyImageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyVideoCombined {
    pub object: LegacyObjectSchema,
    pub metadata: LegacyVideoMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyAlbumCombined {
    pub object: LegacyObjectSchema,
    pub metadata: LegacyAlbumMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyObjectSchema {
    pub id: ArrayString<64>,
    pub obj_type: LegacyObjectType,
    pub pending: bool,
    pub thumbhash: Option<Vec<u8>>,
    pub description: Option<String>,
    pub tags: HashSet<String>,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub is_trashed: bool,
    pub update_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum LegacyObjectType {
    Image,
    Video,
    Album,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyImageMetadata {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub phash: Option<Vec<u8>>,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
    pub alias: Vec<LegacyFileModify>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyVideoMetadata {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub duration: f64,
    pub albums: HashSet<ArrayString<64>>,
    pub exif_vec: BTreeMap<String, String>,
    pub alias: Vec<LegacyFileModify>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyAlbumMetadata {
    pub id: ArrayString<64>,
    pub title: Option<String>,
    pub created_time: i64,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub last_modified_time: i64,
    pub cover: Option<ArrayString<64>>,
    pub item_count: usize,
    pub item_size: u64,
    pub share_list: HashMap<ArrayString<64>, LegacyShare>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyFileModify {
    pub file: String,
    pub modified: i64,
    pub scan_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LegacyShare {
    pub url: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: i64,
}

impl Value for LegacyAbstractData {
    type SelfType<'a> = Self;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bitcode::decode(data).expect("failed to decode legacy V5 AbstractData")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a> {
        bitcode::encode(value)
    }

    fn type_name() -> TypeName {
        TypeName::new("AbstractData")
    }
}
