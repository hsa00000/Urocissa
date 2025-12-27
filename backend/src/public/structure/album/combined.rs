use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::metadata::AlbumMetadata;
use crate::public::db::tree::TREE;
use crate::public::structure::abstract_data::AbstractData;
use crate::public::structure::object::ObjectSchema;

/// Combined Album data with Object and Metadata
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
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
    timestamp: u128,
}

impl AlbumCombined {
    pub fn set_cover(&mut self, cover_data: &AbstractData) {
        self.metadata.cover = Some(cover_data.hash());
        self.object.thumbhash = cover_data.thumbhash().cloned();
    }

    fn set_cover_from_info(&mut self, info: &MediaItemInfo) {
        self.metadata.cover = Some(info.hash);
        self.object.thumbhash = info.thumbhash.clone();
    }

    pub fn self_update(&mut self) {
        // Acquire a read lock on the in-memory tree
        let ref_data = TREE.in_memory.read().unwrap();

        // Collect relevant media items (Image/Video) along with their info
        let mut data_in_album: Vec<MediaItemInfo> = ref_data
            .par_iter()
            .filter_map(
                |database_timestamp| match &database_timestamp.abstract_data {
                    AbstractData::Image(img) => {
                        // Check if in this album and not trashed
                        if img.metadata.albums.contains(&self.object.id) && !img.object.is_trashed {
                            Some(MediaItemInfo {
                                hash: img.object.id,
                                size: img.metadata.size,
                                thumbhash: img.object.thumbhash.clone(),
                                timestamp: database_timestamp.timestamp,
                            })
                        } else {
                            None
                        }
                    }
                    AbstractData::Video(vid) => {
                        // Check if in this album and not trashed
                        if vid.metadata.albums.contains(&self.object.id) && !vid.object.is_trashed {
                            Some(MediaItemInfo {
                                hash: vid.object.id,
                                size: vid.metadata.size,
                                thumbhash: vid.object.thumbhash.clone(),
                                timestamp: database_timestamp.timestamp,
                            })
                        } else {
                            None
                        }
                    }
                    AbstractData::Album(_) => None,
                },
            )
            .collect();

        // If there are no items in the album, there's nothing to set
        if data_in_album.is_empty() {
            self.metadata.start_time = None;
            self.metadata.end_time = None;
            self.metadata.cover = None;
            self.object.thumbhash = None;
            self.metadata.item_count = 0;
            self.metadata.item_size = 0;
            return;
        }

        // Sort by timestamp descending (newest first)
        data_in_album.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Set metadata from the sorted list
        self.metadata.start_time = data_in_album.last().map(|info| info.timestamp as i64);
        self.metadata.end_time = data_in_album.first().map(|info| info.timestamp as i64);
        self.metadata.item_count = data_in_album.len();
        self.metadata.item_size = data_in_album.iter().map(|info| info.size).sum();

        // Update last_modified_time
        self.metadata.last_modified_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Set cover if not already set
        if self.metadata.cover.is_none() {
            if let Some(first_info) = data_in_album.first() {
                self.set_cover_from_info(first_info);
            }
        } else {
            // Check if current cover is still in the album, if not update it
            let current_cover = self.metadata.cover.unwrap();
            let cover_still_in_album = data_in_album.iter().any(|info| info.hash == current_cover);
            if !cover_still_in_album {
                if let Some(first_info) = data_in_album.first() {
                    self.set_cover_from_info(first_info);
                }
            }
        }
    }
}
