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
pub struct FacetValueInfo {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SearchFacets {
    pub tags: Vec<FacetValueInfo>,
    pub makes: Vec<FacetValueInfo>,
    pub models: Vec<FacetValueInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct TreeListSnapshot {
    pub tags: Vec<FacetValueInfo>,
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
            .map(|(value, count)| FacetValueInfo { value, count })
            .collect::<Vec<_>>();
        tags.sort_unstable_by(|left, right| left.value.cmp(&right.value));
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
    pub fn read_tags(&self) -> Vec<FacetValueInfo> {
        let start_time = Instant::now();
        let state = self.state.read().unwrap();
        let tags = exact_facet_values(state.query.tags.iter());
        crate::perf_timing!(
            "get_list.read_tags",
            start_time,
            "Read {} tags from cache",
            tags.len()
        );
        tags
    }

    pub fn read_search_facets(&self) -> SearchFacets {
        let start_time = Instant::now();
        let state = self.state.read().unwrap();
        let facets = SearchFacets {
            tags: exact_facet_values(state.query.tags.iter()),
            makes: camera_facet_values(state.query.makes.iter()),
            models: camera_facet_values(state.query.models.iter()),
        };
        crate::perf_timing!(
            "get_list.read_search_facets",
            start_time,
            "Read {} tags, {} makes, and {} models from cache",
            facets.tags.len(),
            facets.makes.len(),
            facets.models.len()
        );
        facets
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

fn exact_facet_values<'a>(
    values: impl Iterator<Item = (&'a String, &'a super::state::OrdinalSet)>,
) -> Vec<FacetValueInfo> {
    let mut facets = values
        .map(|(value, members)| FacetValueInfo {
            value: value.clone(),
            count: members.len(),
        })
        .collect::<Vec<_>>();
    facets.sort_unstable_by(|left, right| left.value.cmp(&right.value));
    facets
}

fn camera_facet_values<'a>(
    values: impl Iterator<Item = (&'a String, &'a super::state::OrdinalSet)>,
) -> Vec<FacetValueInfo> {
    let mut counts = HashMap::<String, usize>::new();
    for (raw_value, members) in values {
        let Some(value) = normalize_camera_facet_value(raw_value) else {
            continue;
        };
        let count = counts.entry(value).or_default();
        *count = count.saturating_add(members.len());
    }

    let mut facets = counts
        .into_iter()
        .map(|(value, count)| FacetValueInfo { value, count })
        .collect::<Vec<_>>();
    facets.sort_unstable_by(|left, right| left.value.cmp(&right.value));
    facets
}

fn normalize_camera_facet_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let unquoted = if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
    .trim();
    (!unquoted.is_empty()).then(|| unquoted.to_owned())
}

#[cfg(test)]
mod search_facet_tests {
    use super::{FacetValueInfo, SearchFacets, camera_facet_values, normalize_camera_facet_value};
    use crate::public::db::tree::state::OrdinalSet;
    use std::collections::HashMap;

    #[test]
    fn normalizes_camera_values_without_touching_inner_content() {
        assert_eq!(
            normalize_camera_facet_value("  \" Canon EOS \"  ").as_deref(),
            Some("Canon EOS")
        );
        assert_eq!(
            normalize_camera_facet_value("  Nikon  ").as_deref(),
            Some("Nikon")
        );
        assert_eq!(normalize_camera_facet_value(" \"  \" "), None);
        assert_eq!(
            normalize_camera_facet_value("\"Sony").as_deref(),
            Some("\"Sony")
        );
    }

    #[test]
    fn keeps_ascii_case_variants_separate_after_display_normalization() {
        let values = HashMap::from([
            (
                "  \"Canon\"  ".to_owned(),
                OrdinalSet::from_ordinals([0, 1], 16),
            ),
            ("CANON".to_owned(), OrdinalSet::from_ordinals([2], 16)),
            ("Canon".to_owned(), OrdinalSet::from_ordinals([3, 4, 5], 16)),
            ("nikon".to_owned(), OrdinalSet::from_ordinals([6], 16)),
            ("NIKON".to_owned(), OrdinalSet::from_ordinals([7], 16)),
            (" \" \" ".to_owned(), OrdinalSet::from_ordinals([8], 16)),
        ]);

        assert_eq!(
            camera_facet_values(values.iter()),
            vec![
                FacetValueInfo {
                    value: "CANON".to_owned(),
                    count: 1,
                },
                FacetValueInfo {
                    value: "Canon".to_owned(),
                    count: 5,
                },
                FacetValueInfo {
                    value: "NIKON".to_owned(),
                    count: 1,
                },
                FacetValueInfo {
                    value: "nikon".to_owned(),
                    count: 1,
                },
            ]
        );
    }

    #[test]
    fn serializes_the_unified_search_facet_contract() {
        let facets = SearchFacets {
            tags: vec![FacetValueInfo {
                value: "family".to_owned(),
                count: 2,
            }],
            makes: vec![
                FacetValueInfo {
                    value: "CANON".to_owned(),
                    count: 1,
                },
                FacetValueInfo {
                    value: "Canon".to_owned(),
                    count: 3,
                },
            ],
            models: vec![],
        };

        assert_eq!(
            serde_json::to_value(facets).unwrap(),
            serde_json::json!({
                "tags": [{ "value": "family", "count": 2 }],
                "makes": [
                    { "value": "CANON", "count": 1 },
                    { "value": "Canon", "count": 3 }
                ],
                "models": []
            })
        );
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
                .find(|tag| tag.value == "shared")
                .unwrap()
                .count,
            2
        );
        assert_eq!(
            snapshot
                .tags
                .iter()
                .find(|tag| tag.value == "image-only")
                .unwrap()
                .count,
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
