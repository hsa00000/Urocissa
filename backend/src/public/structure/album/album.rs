use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::combined::AlbumCombined;
use super::metadata::AlbumMetadata;
use super::share::Share;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::object::{ObjectSchema, ObjectType};

#[derive(Debug, Clone, Deserialize, Default, Serialize, Decode, Encode, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: ArrayString<64>,
    pub title: Option<String>,
    pub created_time: u128,
    pub start_time: Option<u128>,
    pub end_time: Option<u128>,
    pub last_modified_time: u128,
    pub cover: Option<ArrayString<64>>,
    pub thumbhash: Option<Vec<u8>>,
    pub share_list: HashMap<ArrayString<64>, Share>,
    pub tag: HashSet<String>,
    pub item_count: usize,
    pub item_size: u64,
    pub pending: bool,
}

impl Album {
    pub fn new(id: ArrayString<64>, title: Option<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Self {
            id,
            title,
            created_time: timestamp,
            cover: None,
            thumbhash: None,
            share_list: HashMap::new(),
            tag: HashSet::new(),
            start_time: None,
            end_time: None,
            last_modified_time: timestamp,
            item_count: 0,
            item_size: 0,
            pending: false,
        }
    }

    /// Convert Album to AbstractData::Album(AlbumCombined)
    pub fn into_abstract_data(self) -> AbstractData {
        // Create ObjectSchema
        let object = ObjectSchema {
            id: self.id,
            obj_type: ObjectType::Album,
            pending: self.pending,
            thumbhash: self.thumbhash,
            description: None,
            tags: self.tag,
            is_favorite: false,
            is_archived: false,
            is_trashed: false,
        };

        // Create AlbumMetadata
        let metadata = AlbumMetadata {
            id: self.id,
            title: self.title,
            created_time: self.created_time as i64,
            start_time: self.start_time.map(|t| t as i64),
            end_time: self.end_time.map(|t| t as i64),
            last_modified_time: self.last_modified_time as i64,
            cover: self.cover,
            item_count: self.item_count,
            item_size: self.item_size,
            share_list: self.share_list,
        };

        AbstractData::Album(AlbumCombined { object, metadata })
    }
}
