use std::time::Instant;

use chrono::{Datelike, TimeZone, Utc};
use redb::ReadableDatabase;

use super::{SCROLLBAR_METADATA_TABLE, TreeSnapshot};
use crate::public::structure::response::row::ScrollBarData;
use crate::storage::codec;

#[cfg(test)]
pub fn build_scrollbar(timestamps: impl IntoIterator<Item = i64>) -> Vec<ScrollBarData> {
    let mut result = Vec::new();
    let mut last_year_month = None;
    for (index, timestamp) in timestamps.into_iter().enumerate() {
        push_boundary(&mut result, &mut last_year_month, index, timestamp);
    }
    result
}

fn push_boundary(
    result: &mut Vec<ScrollBarData>,
    last_year_month: &mut Option<(i32, u32)>,
    index: usize,
    timestamp: i64,
) {
    let year_month = timestamp_year_month(timestamp);
    if *last_year_month == Some(year_month) {
        return;
    }
    *last_year_month = Some(year_month);
    result.push(ScrollBarData {
        #[allow(clippy::cast_sign_loss)]
        year: year_month.0 as usize,
        month: year_month.1 as usize,
        index,
    });
}

pub(crate) fn timestamp_year_month(timestamp: i64) -> (i32, u32) {
    let datetime = Utc
        .timestamp_millis_opt(timestamp)
        .single()
        .expect("record timestamp must be representable");
    (datetime.year(), datetime.month())
}

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: i64) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        if let Some(snapshot) = self.in_memory.get(&timestamp) {
            let result = snapshot.scrollbar.clone();
            crate::perf_timing!(
                "tree_snapshot.scrollbar.memory_hit",
                start_time,
                "Read scrollbar metadata from memory"
            );
            crate::perf_timing!(
                "tree_snapshot.generate_scrollbar",
                start_time,
                "Read cached scrollbar"
            );
            return result;
        }

        let disk_start = Instant::now();
        match read_scrollbar_metadata(self.in_disk, timestamp) {
            Ok(Some(result)) => {
                crate::perf_timing!(
                    "tree_snapshot.scrollbar.disk_hit",
                    disk_start,
                    "Read scrollbar metadata from disk"
                );
                crate::perf_timing!(
                    "tree_snapshot.generate_scrollbar",
                    start_time,
                    "Read cached scrollbar"
                );
                return result;
            }
            Ok(None) => {}
            Err(error) => {
                log::warn!("failed to read scrollbar metadata for {timestamp}: {error:#}");
            }
        }

        let fallback_start = Instant::now();
        let result = match self.build_scrollbar_from_snapshot(timestamp) {
            Ok(result) => result,
            Err(error) => {
                log::warn!("failed to rebuild scrollbar metadata for {timestamp}: {error:#}");
                Vec::new()
            }
        };
        if let Err(error) = self.repair_scrollbar_metadata(timestamp, &result) {
            log::warn!("failed to repair scrollbar metadata for {timestamp}: {error:#}");
        }
        crate::perf_timing!(
            "tree_snapshot.scrollbar.fallback",
            fallback_start,
            "Rebuild missing scrollbar metadata"
        );
        crate::perf_timing!(
            "tree_snapshot.generate_scrollbar",
            start_time,
            "Generate scrollbar"
        );
        result
    }

    fn build_scrollbar_from_snapshot(
        &'static self,
        timestamp: i64,
    ) -> anyhow::Result<Vec<ScrollBarData>> {
        let snapshot = self.read_tree_snapshot(timestamp)?;
        let mut result = Vec::new();
        let mut last_year_month = None;
        snapshot.for_each_timestamp(|index, timestamp| {
            push_boundary(&mut result, &mut last_year_month, index, timestamp);
            Ok(())
        })?;
        Ok(result)
    }

    fn repair_scrollbar_metadata(
        &self,
        timestamp: i64,
        scrollbar: &[ScrollBarData],
    ) -> anyhow::Result<()> {
        write_scrollbar_metadata(self.in_disk, timestamp, scrollbar)
    }
}

fn read_scrollbar_metadata(
    database: &redb::Database,
    timestamp: i64,
) -> anyhow::Result<Option<Vec<ScrollBarData>>> {
    let read_txn = database.begin_read()?;
    let table = read_txn.open_table(SCROLLBAR_METADATA_TABLE)?;
    let Some(value) = table.get(timestamp)? else {
        return Ok(None);
    };
    Ok(Some(codec::decode(value.value())?))
}

fn write_scrollbar_metadata(
    database: &redb::Database,
    timestamp: i64,
    scrollbar: &[ScrollBarData],
) -> anyhow::Result<()> {
    let bytes = codec::encode(scrollbar);
    let write_txn = database.begin_write()?;
    {
        let mut table = write_txn.open_table(SCROLLBAR_METADATA_TABLE)?;
        table.insert(timestamp, bytes.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(year: i32, month: u32, day: u32) -> i64 {
        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn scrollbar_boundaries_cover_empty_month_and_year_transitions() {
        assert!(build_scrollbar([]).is_empty());
        let result = build_scrollbar([
            timestamp(2026, 12, 31),
            timestamp(2026, 12, 1),
            timestamp(2026, 11, 1),
            timestamp(2025, 12, 1),
        ]);
        assert_eq!(
            result,
            vec![
                ScrollBarData {
                    year: 2026,
                    month: 12,
                    index: 0,
                },
                ScrollBarData {
                    year: 2026,
                    month: 11,
                    index: 2,
                },
                ScrollBarData {
                    year: 2025,
                    month: 12,
                    index: 3,
                },
            ]
        );
    }

    #[test]
    fn scrollbar_metadata_round_trips_and_reports_corruption() {
        let directory = tempfile::tempdir().unwrap();
        let database = redb::Database::create(directory.path().join("snapshot.redb")).unwrap();
        let expected = build_scrollbar([timestamp(2026, 7, 18), timestamp(2026, 6, 1)]);
        write_scrollbar_metadata(&database, 42, &expected).unwrap();
        assert_eq!(
            read_scrollbar_metadata(&database, 42).unwrap(),
            Some(expected)
        );

        let write_txn = database.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(SCROLLBAR_METADATA_TABLE).unwrap();
            table.insert(43, &[0xff, 0x00][..]).unwrap();
        }
        write_txn.commit().unwrap();
        assert!(read_scrollbar_metadata(&database, 43).is_err());
    }
}
