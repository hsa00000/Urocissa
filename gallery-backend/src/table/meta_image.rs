use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

pub const META_IMAGE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("meta_image");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadataSchema {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub phash: Option<Vec<u8>>,
}
