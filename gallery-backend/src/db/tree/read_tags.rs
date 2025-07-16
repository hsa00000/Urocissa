use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};

use crate::{constant::redb::ALBUM_TABLE, public::structure::album::Album};

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

        let tag_infos: Vec<TagInfo> = tag_counts
            .par_iter()
            .map(|entry| TagInfo {
                tag: entry.key().clone(),
                number: entry.value().load(Ordering::Relaxed),
            })
            .collect();
        tag_infos
    }
    pub fn read_albums(&'static self) -> Vec<Album> {
        let txn = self.in_disk.begin_read().unwrap();

        let album_table = txn.open_table(ALBUM_TABLE).unwrap();

        album_table
            .iter()
            .unwrap()
            .par_bridge()
            .map(|result| {
                let (_, access_guard) = result.unwrap();
                let album = access_guard.value();
                album
            })
            .collect()
    }
}
