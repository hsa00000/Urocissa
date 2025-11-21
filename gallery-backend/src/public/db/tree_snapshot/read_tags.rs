use super::TreeSnapshot;
use crate::{public::db::tree::read_tags::TagInfo, public::structure::database_struct::database::definition::Database};
use anyhow::{Context, Result};
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use rusqlite::Connection;
use std::sync::atomic::{AtomicUsize, Ordering};
impl TreeSnapshot {
    pub fn read_tags(&self) -> Result<Vec<TagInfo>> {
        // Concurrent counter for each tag
        let tag_counts: DashMap<String, AtomicUsize> = DashMap::new();

        let conn = crate::public::db::sqlite::DB_POOL.get().context("Failed to get DB connection")?;
        let mut stmt = conn.prepare("SELECT * FROM database").context("Failed to prepare statement")?;
        let rows = stmt.query_map([], |row| Database::from_row(row)).context("Failed to query database")?;

        let databases: Vec<Database> = rows.collect::<Result<Vec<_>, _>>().context("Failed to collect databases")?;

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
