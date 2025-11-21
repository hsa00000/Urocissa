use super::TreeSnapshot;
use crate::{
    public::db::tree::TREE, public::db::tree::read_tags::TagInfo,
};
use anyhow::Result;
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::sync::atomic::{AtomicUsize, Ordering};
impl TreeSnapshot {
    pub fn read_tags(&self) -> Result<Vec<TagInfo>> {
        // Concurrent counter for each tag
        let tag_counts: DashMap<String, AtomicUsize> = DashMap::new();

        let databases = TREE.load_all_databases_from_db()?;

        databases.par_iter().try_for_each(|db| -> Result<()> {
            for tag in &db.tag {
                tag_counts
                    .entry(tag.clone())
                    .or_insert_with(|| AtomicUsize::new(0))
                    .fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        })?;

        let tag_infos = tag_counts
            .par_iter()
            .map(|e| TagInfo {
                tag: e.key().clone(),
                number: e.value().load(Ordering::Relaxed),
            })
            .collect();

        Ok(tag_infos)
    }
}
