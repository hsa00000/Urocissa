use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};

use super::metadata::AlbumMetadata;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::object::ObjectSchema;

/// Combined Album data with Object and Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumCombined {
    #[serde(flatten)]
    pub object: ObjectSchema,
    #[serde(flatten)]
    pub metadata: AlbumMetadata,
}

/// A helper struct to hold media item info for album calculations
struct MediaItemInfo {
    hash: ArrayString<64>,
    size: u64,
    thumbhash: Option<Vec<u8>>,
    cache_version: u32,
    timestamp: i64,
}

impl AlbumCombined {
    pub fn set_cover(&mut self, cover_data: &AbstractData) {
        self.metadata.cover = Some(cover_data.hash());
        self.object.thumbhash = cover_data.thumbhash().cloned();
        self.object.cache_version = cover_data.cache_version();
    }

    fn set_cover_from_info(&mut self, info: &MediaItemInfo) {
        self.metadata.cover = Some(info.hash);
        self.object.thumbhash.clone_from(&info.thumbhash);
        self.object.cache_version = info.cache_version;
    }

    pub fn self_update(&mut self, changed_at: i64) {
        let state = TREE.state.read().unwrap();
        let mut data_in_album = state
            .query
            .albums
            .get(&self.object.id)
            .into_iter()
            .flat_map(|members| members.iter())
            .filter(|ordinal| !state.query.trashed.contains(*ordinal))
            .filter_map(|ordinal| state.slot_for_ordinal(ordinal))
            .filter_map(|slot_ref| state.get(slot_ref))
            .filter(|record| {
                record.object_type != crate::public::structure::object::ObjectType::Album
            })
            .map(|record| MediaItemInfo {
                hash: record.id,
                size: record.size,
                thumbhash: record.thumbhash_vec(),
                cache_version: record.cache_version,
                timestamp: record.timestamp,
            })
            .collect::<Vec<_>>();

        // If there are no items in the album, there's nothing to set
        if data_in_album.is_empty() {
            self.metadata.start_time = None;
            self.metadata.end_time = None;
            self.metadata.cover = None;
            self.object.thumbhash = None;
            self.object.cache_version = 0;
            self.metadata.item_count = 0;
            self.metadata.item_size = 0;
            return;
        }

        // Sort by timestamp descending (newest first)
        data_in_album.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Set metadata from the sorted list
        self.metadata.start_time = data_in_album.last().map(|info| info.timestamp);
        self.metadata.end_time = data_in_album.first().map(|info| info.timestamp);
        self.metadata.item_count = data_in_album.len();
        self.metadata.item_size = data_in_album.iter().map(|info| info.size).sum();

        // Update last_modified_time
        self.metadata.last_modified_time = changed_at;

        // Set cover if not already set
        if self.metadata.cover.is_none() {
            if let Some(first_info) = data_in_album.first() {
                self.set_cover_from_info(first_info);
            }
        } else {
            // Check if current cover is still in the album, if not update it
            let current_cover = self.metadata.cover.unwrap();
            if let Some(current_info) = data_in_album.iter().find(|info| info.hash == current_cover)
            {
                self.set_cover_from_info(current_info);
            } else if let Some(first_info) = data_in_album.first() {
                self.set_cover_from_info(first_info);
            }
        }
    }
}
