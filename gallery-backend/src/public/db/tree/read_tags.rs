use std::sync::atomic::{AtomicUsize, Ordering};

use crate::public::structure::abstract_data::AbstractData;
use crate::table::album::AlbumCombined;
use anyhow::Result;
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
                let tags = match database_timestamp {
                    AbstractData::Image(i) => &i.object.tags,
                    AbstractData::Video(v) => &v.object.tags,
                    AbstractData::Album(a) => &a.object.tags,
                };
                for tag in tags {
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
    pub fn read_albums(&self) -> Result<Vec<AlbumCombined>> {
        let conn = self.get_connection()?;
        Ok(AlbumCombined::get_all(&conn)?)
    }
}
