use super::TreeSnapshot;
use crate::{operations::open_db::open_data_table, public::db::tree::read_tags::TagInfo};
use anyhow::{Context, Result};
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use std::sync::atomic::{AtomicUsize, Ordering};

impl TreeSnapshot {
    pub fn read_tags() -> Result<Vec<TagInfo>> {
        // Concurrent counter for each tag
        let tag_counts: DashMap<String, AtomicUsize> = DashMap::new();

        // Open the durable records table through the storage repository.
        let data_table = open_data_table();

        // Walk the table in parallel; stop on first error
        data_table
            .iter()
            .context("Create iterator over records table failed")?
            .par_bridge()
            .try_for_each(|entry| -> Result<()> {
                let (_, data) = entry.context("Read table row failed")?;
                let abstract_data = data.value();

                // Count regular tags only
                for tag in abstract_data.tag() {
                    tag_counts
                        .entry(tag.clone())
                        .or_insert_with(|| AtomicUsize::new(0))
                        .fetch_add(1, Ordering::Relaxed);
                }

                Ok(())
            })?;

        let tag_infos: Vec<TagInfo> = tag_counts
            .par_iter()
            .map(|e| TagInfo {
                tag: e.key().clone(),
                number: e.value().load(Ordering::Relaxed),
            })
            .collect();

        Ok(tag_infos)
    }
}
