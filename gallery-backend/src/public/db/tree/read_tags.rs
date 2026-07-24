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
        self.read_search_facets_with_trash_scope(None)
    }

    pub fn read_search_facets_for_trash_state(&self, trashed: bool) -> SearchFacets {
        self.read_search_facets_with_trash_scope(Some(trashed))
    }

    fn read_search_facets_with_trash_scope(&self, trashed: Option<bool>) -> SearchFacets {
        let start_time = Instant::now();
        let state = self.state.read().unwrap();
        let facets = SearchFacets {
            tags: exact_facet_values_for_scope(
                state.query.tags.iter(),
                &state.query.trashed,
                trashed,
            ),
            makes: camera_facet_values_for_scope(
                state.query.makes.iter(),
                &state.query.trashed,
                trashed,
            ),
            models: camera_facet_values_for_scope(
                state.query.models.iter(),
                &state.query.trashed,
                trashed,
            ),
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
    exact_facet_values_for_scope(values, &super::state::DenseBitmap::default(), None)
}

fn exact_facet_values_for_scope<'a>(
    values: impl Iterator<Item = (&'a String, &'a super::state::OrdinalSet)>,
    trashed_members: &super::state::DenseBitmap,
    trashed: Option<bool>,
) -> Vec<FacetValueInfo> {
    let mut facets = values
        .filter_map(|(value, members)| {
            let count = count_members_in_trash_scope(members, trashed_members, trashed);
            (count > 0).then(|| FacetValueInfo {
                value: value.clone(),
                count,
            })
        })
        .collect::<Vec<_>>();
    facets.sort_unstable_by(|left, right| left.value.cmp(&right.value));
    facets
}

fn camera_facet_values<'a>(
    values: impl Iterator<Item = (&'a String, &'a super::state::OrdinalSet)>,
) -> Vec<FacetValueInfo> {
    camera_facet_values_for_scope(values, &super::state::DenseBitmap::default(), None)
}

fn camera_facet_values_for_scope<'a>(
    values: impl Iterator<Item = (&'a String, &'a super::state::OrdinalSet)>,
    trashed_members: &super::state::DenseBitmap,
    trashed: Option<bool>,
) -> Vec<FacetValueInfo> {
    #[derive(Default)]
    struct Aggregate {
        count: usize,
        spellings: HashMap<String, usize>,
    }

    let mut aggregates = HashMap::<String, Aggregate>::new();
    for (raw_value, members) in values {
        let Some(value) = normalize_camera_facet_value(raw_value) else {
            continue;
        };
        let member_count = count_members_in_trash_scope(members, trashed_members, trashed);
        if member_count == 0 {
            continue;
        }
        let aggregate = aggregates.entry(value.to_ascii_lowercase()).or_default();
        aggregate.count = aggregate.count.saturating_add(member_count);
        let spelling_count = aggregate.spellings.entry(value).or_default();
        *spelling_count = spelling_count.saturating_add(member_count);
    }

    let mut facets = aggregates
        .into_iter()
        .filter_map(|(_, aggregate)| {
            let value = aggregate
                .spellings
                .into_iter()
                // Prefer the most common spelling, then the lexicographically
                // smallest spelling so HashMap iteration cannot affect output.
                .max_by(|(left_value, left_count), (right_value, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_value.cmp(left_value))
                })?
                .0;
            Some(FacetValueInfo {
                value,
                count: aggregate.count,
            })
        })
        .collect::<Vec<_>>();
    facets.sort_unstable_by(|left, right| left.value.cmp(&right.value));
    facets
}

fn count_members_in_trash_scope(
    members: &super::state::OrdinalSet,
    trashed_members: &super::state::DenseBitmap,
    trashed: Option<bool>,
) -> usize {
    match trashed {
        None => members.len(),
        Some(expected) => members
            .iter()
            .filter(|ordinal| trashed_members.contains(*ordinal) == expected)
            .count(),
    }
}

fn normalize_camera_facet_value(value: &str) -> Option<String> {
    let mut trimmed = value.trim();
    while let Some(without_empty) = trimmed.strip_suffix("\"\"") {
        let without_empty = without_empty.trim_end();
        let Some(without_comma) = without_empty.strip_suffix(',') else {
            break;
        };
        trimmed = without_comma.trim_end();
    }
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
    use super::{
        FacetValueInfo, SearchFacets, camera_facet_values, camera_facet_values_for_scope,
        exact_facet_values, exact_facet_values_for_scope, normalize_camera_facet_value,
    };
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
        assert_eq!(
            normalize_camera_facet_value(" \"OPPO\", \"\",\"\",  \"\" ").as_deref(),
            Some("OPPO")
        );
        assert_eq!(normalize_camera_facet_value("\"\", \"\", \"\""), None);
    }

    #[test]
    fn merges_ascii_case_variants_and_uses_a_stable_representative() {
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
                    value: "Canon".to_owned(),
                    count: 6,
                },
                FacetValueInfo {
                    value: "NIKON".to_owned(),
                    count: 2,
                },
            ]
        );
    }

    #[test]
    fn regular_tags_remain_case_sensitive() {
        let values = HashMap::from([
            ("Family".to_owned(), OrdinalSet::from_ordinals([0], 16)),
            ("family".to_owned(), OrdinalSet::from_ordinals([1, 2], 16)),
        ]);

        assert_eq!(
            exact_facet_values(values.iter()),
            vec![
                FacetValueInfo {
                    value: "Family".to_owned(),
                    count: 1,
                },
                FacetValueInfo {
                    value: "family".to_owned(),
                    count: 2,
                },
            ]
        );
    }

    #[test]
    fn filters_exact_facets_by_trash_state_and_omits_empty_values() {
        let values = HashMap::from([
            ("shared".to_owned(), OrdinalSet::from_ordinals([0, 1], 16)),
            ("active-only".to_owned(), OrdinalSet::from_ordinals([2], 16)),
            (
                "trashed-only".to_owned(),
                OrdinalSet::from_ordinals([3], 16),
            ),
        ]);
        let mut trashed_members = super::super::state::DenseBitmap::default();
        trashed_members.set(1, true);
        trashed_members.set(3, true);

        assert_eq!(
            exact_facet_values_for_scope(values.iter(), &trashed_members, Some(false)),
            vec![
                FacetValueInfo {
                    value: "active-only".to_owned(),
                    count: 1,
                },
                FacetValueInfo {
                    value: "shared".to_owned(),
                    count: 1,
                },
            ]
        );
        assert_eq!(
            exact_facet_values_for_scope(values.iter(), &trashed_members, Some(true)),
            vec![
                FacetValueInfo {
                    value: "shared".to_owned(),
                    count: 1,
                },
                FacetValueInfo {
                    value: "trashed-only".to_owned(),
                    count: 1,
                },
            ]
        );
    }

    #[test]
    fn filters_camera_facets_before_case_variant_aggregation() {
        let values = HashMap::from([
            ("Canon".to_owned(), OrdinalSet::from_ordinals([0, 1], 16)),
            ("CANON".to_owned(), OrdinalSet::from_ordinals([2], 16)),
            ("Nikon".to_owned(), OrdinalSet::from_ordinals([3], 16)),
        ]);
        let mut trashed_members = super::super::state::DenseBitmap::default();
        trashed_members.set(1, true);
        trashed_members.set(2, true);
        trashed_members.set(3, true);

        assert_eq!(
            camera_facet_values_for_scope(values.iter(), &trashed_members, Some(false)),
            vec![FacetValueInfo {
                value: "Canon".to_owned(),
                count: 1,
            }]
        );
        assert_eq!(
            camera_facet_values_for_scope(values.iter(), &trashed_members, Some(true)),
            vec![
                FacetValueInfo {
                    value: "CANON".to_owned(),
                    count: 2,
                },
                FacetValueInfo {
                    value: "Nikon".to_owned(),
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
            makes: vec![FacetValueInfo {
                value: "Canon".to_owned(),
                count: 4,
            }],
            models: vec![],
        };

        assert_eq!(
            serde_json::to_value(facets).unwrap(),
            serde_json::json!({
                "tags": [{ "value": "family", "count": 2 }],
                "makes": [{ "value": "Canon", "count": 4 }],
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
