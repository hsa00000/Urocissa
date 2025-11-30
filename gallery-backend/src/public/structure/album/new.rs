use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

use arrayvec::ArrayString;

use super::{Album, ResolvedShare, Share};

impl Album {
    pub fn new(id: ArrayString<64>, title: Option<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Self {
            id: id,
            title: title,
            created_time: timestamp,
            cover: None,
            thumbhash: None,
            user_defined_metadata: HashMap::new(),
            tag: HashSet::new(),
            width: 300,
            height: 300,
            start_time: None,
            end_time: None,
            last_modified_time: timestamp,
            item_count: 0,
            item_size: 0,
            pending: false,
        }
    }
}

impl ResolvedShare {
    pub fn new(album_id: ArrayString<64>, album_title: Option<String>, share: Share) -> Self {
        Self {
            share,
            album_id,
            album_title,
        }
    }
}
