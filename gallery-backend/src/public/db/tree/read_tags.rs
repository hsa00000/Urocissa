use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    public::error::{AppError, ErrorKind, ResultExt}, // Import AppError stuff
    public::structure::{abstract_data::AbstractData, album::AlbumCombined},
};
use anyhow::Result; // Use standard Result or alias? Standard Result<T, E> is fine.
// But we want to return Result<Vec<...>, AppError>
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};

use super::Tree;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct TagInfo {
    pub tag: String,
    pub number: usize,
}

impl Tree {
    pub fn read_tags(&'static self) -> Vec<TagInfo> {
        // ... (unchanged)
        let tag_counts: DashMap<String, AtomicUsize> = DashMap::new();

        self.in_memory
            .read()
            .unwrap()
            .iter()
            .par_bridge()
            .for_each(|database_timestamp| {
                let abstract_data = &database_timestamp.abstract_data;

                // Count regular tags only
                for tag in abstract_data.tag() {
                    let counter = tag_counts
                        .entry(tag.clone())
                        .or_insert_with(|| AtomicUsize::new(0));
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });

        let tag_infos: Vec<TagInfo> = tag_counts
            .par_iter()
            .map(|entry| TagInfo {
                tag: entry.key().clone(),
                number: entry.value().load(Ordering::Relaxed),
            })
            .collect();

        tag_infos
    }

    pub fn read_albums(&self) -> Result<Vec<AlbumCombined>, AppError> {
        let data_table = self
            .store
            .reader()
            .or_raise(|| (ErrorKind::Database, "Failed to begin read transaction"))?;
        data_table
            .iter()
            .or_raise(|| {
                (
                    ErrorKind::Database,
                    "Failed to create iterator over records table",
                )
            })?
            .par_bridge()
            .filter_map(|entry| {
                entry
                    .map(|(_, guard)| {
                        let abstract_data = guard.value();
                        match abstract_data {
                            AbstractData::Album(album) => Some(album),
                            _ => None,
                        }
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()
            .or_raise(|| {
                (
                    ErrorKind::Database,
                    "Failed to collect album records in parallel",
                )
            })
    }
}
