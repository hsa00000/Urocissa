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

#[cfg(test)]
mod tests {
    use super::*;

    const V5_GOLDEN_BYTES: &[u8] = &[
        0, 9, 118, 53, 45, 103, 111, 108, 100, 101, 110, 0, 0, 1, 3, 6, 57, 1, 6, 108, 101, 103,
        97, 99, 121, 0, 1, 0, 1, 0, 133, 255, 255, 255, 255, 255, 255, 255, 9, 118, 53, 45, 103,
        111, 108, 100, 101, 110, 6, 42, 4, 10, 4, 20, 3, 106, 112, 103, 0, 0, 0, 1, 5, 97, 46, 106,
        112, 103, 6, 1, 6, 2,
    ];

    fn golden_fixture() -> LegacyAbstractData {
        let id = ArrayString::<64>::from("v5-golden").unwrap();
        LegacyAbstractData::Image(LegacyImageCombined {
            object: LegacyObjectSchema {
                id,
                obj_type: LegacyObjectType::Image,
                pending: false,
                thumbhash: Some(vec![1, 2, 3]),
                description: Some("legacy".to_owned()),
                tags: HashSet::new(),
                is_favorite: true,
                is_archived: false,
                is_trashed: true,
                update_at: -123,
            },
            metadata: LegacyImageMetadata {
                id,
                size: 42,
                width: 10,
                height: 20,
                ext: "jpg".to_owned(),
                phash: None,
                albums: HashSet::new(),
                exif_vec: BTreeMap::new(),
                alias: vec![LegacyFileModify {
                    file: "a.jpg".to_owned(),
                    modified: 1,
                    scan_time: 2,
                }],
            },
        })
    }

    #[test]
    fn v5_golden_bytes_decode() {
        let decoded: LegacyAbstractData = bitcode::decode(V5_GOLDEN_BYTES).unwrap();
        let LegacyAbstractData::Image(decoded) = decoded else {
            panic!("V5 golden record changed variant");
        };
        let LegacyAbstractData::Image(expected) = golden_fixture() else {
            unreachable!();
        };

        assert_eq!(decoded.object.id, expected.object.id);
        assert!(matches!(decoded.object.obj_type, LegacyObjectType::Image));
        assert_eq!(decoded.object.thumbhash, expected.object.thumbhash);
        assert_eq!(decoded.object.description, expected.object.description);
        assert_eq!(decoded.object.update_at, expected.object.update_at);
        assert_eq!(decoded.metadata.id, expected.metadata.id);
        assert_eq!(decoded.metadata.size, expected.metadata.size);
        assert_eq!(decoded.metadata.width, expected.metadata.width);
        assert_eq!(decoded.metadata.height, expected.metadata.height);
        assert_eq!(decoded.metadata.ext, expected.metadata.ext);
        assert_eq!(decoded.metadata.alias[0].file, "a.jpg");
    }
}
