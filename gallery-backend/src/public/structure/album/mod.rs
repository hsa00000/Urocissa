use std::collections::{HashMap, HashSet};

use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub mod edit;
pub mod new;

#[derive(Debug, Clone, Deserialize, Default, Serialize, Decode, Encode, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub url: ArrayString<64>,
    pub description: String,
    pub password: Option<String>,
    pub show_metadata: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub exp: u64,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize, Decode, Encode, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedShare {
    pub share: Share,
    pub album_id: ArrayString<64>,
    pub album_title: Option<String>,
}

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
    pub user_defined_metadata: HashMap<String, Vec<String>>,
    pub share_list: HashMap<ArrayString<64>, Share>,
    pub tag: HashSet<String>,
    pub width: u32,
    pub height: u32,
    pub item_count: usize,
    pub item_size: u64,
    pub pending: bool,
}
