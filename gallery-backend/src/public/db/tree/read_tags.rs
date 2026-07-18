use crate::{
    public::error::AppError,
    public::structure::{
        abstract_data::AbstractData, album::AlbumCombined,
        response::database_timestamp::DatabaseTimestamp,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use super::Tree;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct TagInfo {
    pub tag: String,
    pub number: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TreeListSnapshot {
    pub tags: Vec<TagInfo>,
    pub albums: Vec<AlbumCombined>,
}

impl TreeListSnapshot {
    pub fn from_abstract_records(records: &[AbstractData]) -> Self {
        let mut tag_counts = HashMap::<String, usize>::new();
        let mut albums = Vec::new();

        for abstract_data in records {
            for tag in abstract_data.tag() {
                *tag_counts.entry(tag.clone()).or_default() += 1;
            }
            if let AbstractData::Album(album) = abstract_data {
                albums.push(album.clone());
            }
        }

        let mut tags = tag_counts
            .into_iter()
            .map(|(tag, number)| TagInfo { tag, number })
            .collect::<Vec<_>>();
        tags.sort_unstable_by(|left, right| left.tag.cmp(&right.tag));
        albums.sort_unstable_by_key(|album| album.object.id);
        Self { tags, albums }
    }

    pub fn from_records(records: &[DatabaseTimestamp]) -> Self {
        let records = records
            .iter()
            .map(|record| record.abstract_data.clone())
            .collect::<Vec<_>>();
        Self::from_abstract_records(&records)
    }

    fn replace_album(&mut self, album: AlbumCombined) {
        if let Some(existing) = self
            .albums
            .iter_mut()
            .find(|existing| existing.object.id == album.object.id)
        {
            *existing = album;
        } else {
            self.albums.push(album);
            self.albums
                .sort_unstable_by(|left, right| left.object.id.cmp(&right.object.id));
        }
    }

    fn remove_album(&mut self, album_id: &str) {
        self.albums
            .retain(|album| album.object.id.as_str() != album_id);
    }
}

impl Tree {
    pub fn read_tags(&self) -> Vec<TagInfo> {
        let start_time = Instant::now();
        let state = self.state.read().unwrap();
        let mut tags = state
            .query
            .tags
            .iter()
            .map(|(tag, members)| TagInfo {
                tag: tag.clone(),
                number: members.len(),
            })
            .collect::<Vec<_>>();
        tags.sort_unstable_by(|left, right| left.tag.cmp(&right.tag));
        crate::perf_timing!(
            "get_list.read_tags",
            start_time,
            "Read {} tags from cache",
            tags.len()
        );
        tags
    }

    pub fn read_albums(&self) -> Result<Vec<AlbumCombined>, AppError> {
        let start_time = Instant::now();
        let mut albums = self
            .state
            .read()
            .unwrap()
            .albums
            .values()
            .cloned()
            .collect::<Vec<_>>();
        albums.sort_unstable_by_key(|album| album.object.id);
        crate::perf_timing!(
            "get_list.read_albums",
            start_time,
            "Read {} albums from cache",
            albums.len()
        );
        Ok(albums)
    }

    pub fn with_list_snapshot_update<R>(&self, operation: impl FnOnce() -> R) -> R {
        let _update_guard = self.list_snapshot_update_lock.lock().unwrap();
        operation()
    }

    pub fn replace_tree_snapshot(
        &self,
        state: super::state::TreeState,
        list_snapshot: TreeListSnapshot,
    ) {
        *self.state.write().unwrap() = state;
        *self.list_snapshot.write().unwrap() = Some(Arc::new(list_snapshot));
    }

    pub fn refresh_album_snapshot(&self, album_id: &str) -> anyhow::Result<()> {
        self.with_list_snapshot_update(|| {
            let Some(current_snapshot) = self.list_snapshot.read().unwrap().as_ref().cloned()
            else {
                return Ok(());
            };

            let album = self.store.read(|data_table| {
                data_table
                    .get(album_id)
                    .map(|value| value.map(|value| value.into_value()))
            })?;

            let mut next_snapshot = (*current_snapshot).clone();
            match album {
                Some(AbstractData::Album(album)) => {
                    next_snapshot.replace_album(album.clone());
                    self.state
                        .write()
                        .unwrap()
                        .albums
                        .insert(album.object.id, album);
                }
                _ => {
                    next_snapshot.remove_album(album_id);
                    if let Ok(album_id) = arrayvec::ArrayString::<64>::from(album_id) {
                        self.state.write().unwrap().albums.remove(&album_id);
                    }
                }
            }
            *self.list_snapshot.write().unwrap() = Some(Arc::new(next_snapshot));
            Ok(())
        })
    }
}

#[cfg(all(test, feature = "performance-test"))]
mod tests {
    use super::TreeListSnapshot;
    use crate::public::structure::{
        abstract_data::AbstractData, album::Album, response::database_timestamp::DatabaseTimestamp,
    };
    use arrayvec::ArrayString;

    #[test]
    fn builds_tag_counts_and_album_list_from_records() {
        let mut image =
            crate::public::structure::abstract_data::AbstractData::generate_performance_data(1, 7);
        image.tag_mut().insert("shared".to_string());
        image.tag_mut().insert("image-only".to_string());

        let mut video =
            crate::public::structure::abstract_data::AbstractData::generate_performance_data(2, 7);
        video.tag_mut().insert("shared".to_string());

        let album_id = ArrayString::<64>::from("album-one").unwrap();
        let album = Album::new(album_id, Some("Album".to_string())).into_abstract_data();
        let records = vec![
            DatabaseTimestamp::new(image, &[]),
            DatabaseTimestamp::new(video, &[]),
            DatabaseTimestamp::new(album, &[]),
        ];

        let snapshot = TreeListSnapshot::from_records(&records);
        assert_eq!(snapshot.albums.len(), 1);
        assert_eq!(snapshot.albums[0].object.id, album_id);
        assert_eq!(
            snapshot
                .tags
                .iter()
                .find(|tag| tag.tag == "shared")
                .unwrap()
                .number,
            2
        );
        assert_eq!(
            snapshot
                .tags
                .iter()
                .find(|tag| tag.tag == "image-only")
                .unwrap()
                .number,
            1
        );
    }

    #[test]
    fn empty_records_produce_empty_lists() {
        let snapshot = TreeListSnapshot::from_records(&[]);
        assert!(snapshot.tags.is_empty());
        assert!(snapshot.albums.is_empty());
    }

    #[test]
    fn album_patch_replaces_and_removes_entries() {
        let album_id = ArrayString::<64>::from("album-one").unwrap();
        let album = Album::new(album_id, Some("Old".to_string())).into_abstract_data();
        let records = vec![DatabaseTimestamp::new(album, &[])];
        let mut snapshot = TreeListSnapshot::from_records(&records);

        let updated = match records[0].abstract_data.clone() {
            AbstractData::Album(mut album) => {
                album.metadata.title = Some("New".to_string());
                album
            }
            _ => unreachable!(),
        };
        snapshot.replace_album(updated);
        assert_eq!(snapshot.albums[0].metadata.title.as_deref(), Some("New"));

        snapshot.remove_album(album_id.as_str());
        assert!(snapshot.albums.is_empty());
    }
}
