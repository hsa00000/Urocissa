use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const META_ALBUM_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("meta_album");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct AlbumMetadataSchema {
    pub id: ArrayString<64>,
    pub title: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub last_modified_time: i64,
    pub cover: Option<ArrayString<64>>,
    pub user_defined_metadata: HashMap<String, Vec<String>>,
    pub item_count: usize,
    pub item_size: u64,
}

impl AlbumMetadataSchema {
    pub fn new(id: ArrayString<64>, title: Option<String>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        Self {
            id,
            title,
            start_time: None,
            end_time: None,
            last_modified_time: timestamp,
            cover: None,
            user_defined_metadata: HashMap::new(),
            item_count: 0,
            item_size: 0,
        }
    }
}
