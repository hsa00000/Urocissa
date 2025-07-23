use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{public::constant::redb::ALBUM_TABLE, public::structure::album::Album};
use anyhow::{Context, Result};
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use redb::ReadableTable;
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
                for tag in database_timestamp.abstract_data.tag() {
                    let counter = tag_counts
                        .entry(tag.clone())
                        .or_insert_with(|| AtomicUsize::new(0));
                    counter.fetch_add(1, Ordering::Relaxed);
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
    pub fn read_albums(&self) -> Result<Vec<Album>> {
        Ok(
            self.in_disk
                .begin_read()
                .context("Failed to begin read transaction")? // ①
                .open_table(ALBUM_TABLE)
                .context("Failed to open ALBUM_TABLE")? // ②
                .iter()
                .context("Failed to create iterator over ALBUM_TABLE")? // ③
                .par_bridge()
                .map(|entry| entry.map(|(_, guard)| guard.value()))
                .collect::<Result<Vec<_>, _>>()
                .context("Failed to collect album records in parallel")?, // ④
        )
    }
}
