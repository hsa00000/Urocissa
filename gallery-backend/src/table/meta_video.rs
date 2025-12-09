use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

pub const META_VIDEO_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("meta_video");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadataSchema {
    pub id: ArrayString<64>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub duration: f64,
}
