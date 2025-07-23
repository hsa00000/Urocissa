use super::TreeSnapshot;
use crate::{operations::open_db::open_data_table, public::db::tree::read_tags::TagInfo};
use anyhow::{Context, Result};
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use redb::ReadableTable;
use std::sync::atomic::{AtomicUsize, Ordering};
impl TreeSnapshot {
    pub fn read_tags(&self) -> Result<Vec<TagInfo>> {
        // Concurrent counter for each tag
        let tag_counts: DashMap<String, AtomicUsize> = DashMap::new();

        // Begin read‑only transaction and open the DATA_TABLE
        let data_table = open_data_table().context("Open DATA_TABLE failed")?;

        // Walk the table in parallel; stop on first error
        data_table
            .iter()
            .context("Create iterator over DATA_TABLE failed")?
            .par_bridge()
            .try_for_each(|entry| -> Result<()> {
                let (_, data) = entry.context("Read table row failed")?;
                for tag in &data.value().tag {
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
