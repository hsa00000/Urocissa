use std::sync::atomic::{AtomicUsize, Ordering};

use crate::table::album::AlbumSchema;
use anyhow::{Context, Result};
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
        let tag_counts = DashMap::new();

        self.in_memory
            .read()
            .unwrap()
            .iter()
            .par_bridge()
            .for_each(|database_timestamp| {
                if let Some(tags) = database_timestamp.tag() {
                    for tag in tags {
                        let counter = tag_counts
                            .entry(tag.clone())
                            .or_insert_with(|| AtomicUsize::new(0));
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });

        let tag_infos = tag_counts
            .par_iter()
            .map(|entry| TagInfo {
                tag: entry.key().clone(),
                number: entry.value().load(Ordering::Relaxed),
            })
            .collect();
        tag_infos
    }
    pub fn read_albums(&self) -> Result<Vec<AlbumSchema>> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT * FROM album")
            .context("Failed to prepare statement")?;
        let albums = stmt
            .query_map([], |row| AlbumSchema::from_row(row))
            .context("Failed to query albums")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect albums")?;
        Ok(albums)
    }
}
