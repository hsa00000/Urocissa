use super::tree::{MyCow, TreeSnapshot};
use crate::{models::dto::reduced_data::ReducedData, models::entity::row::ScrollBarData};
use chrono::{Datelike, TimeZone, Utc};
use log::info;
use std::time::Instant;

use redb::ReadableDatabase;

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: u128) -> Vec<ScrollBarData> {
        let start_time = Instant::now();

        // 1. 開啟 Transaction
        let txn = self.in_disk.begin_read().unwrap();

        // 2. 傳入 txn
        let tree_snapshot = self.read_tree_snapshot(&txn, &timestamp).unwrap();
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        let mut process_data = |index: usize, data: &ReducedData| {
            // 修改處：使用 timestamp_millis_opt 來解析毫秒時間戳
            let datetime = Utc.timestamp_millis_opt(data.date as i64).unwrap();
            let year = datetime.year();
            let month = datetime.month();
            if last_year != Some(year) || last_month != Some(month) {
                last_year = Some(year);
                last_month = Some(month);
                scroll_bar_data_vec.push(ScrollBarData {
                    year: year as usize,
                    month: month as usize,
                    index: index,
                });
            }
        };

        match tree_snapshot {
            MyCow::DashMap(ref_data) => {
                ref_data.iter().enumerate().for_each(|(index, data)| {
                    process_data(index, data);
                });
            }
            MyCow::Redb(table, ts) => {
                let start = (ts, 0);
                let end = (ts, u64::MAX);
                for (index, entry) in table.range(start..=end).unwrap().enumerate() {
                    let (_, val) = entry.unwrap();
                    if let Ok(data) = bitcode::decode::<ReducedData>(val.value()) {
                        process_data(index, &data);
                    }
                }
            }
        }
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Generate scrollbar");
        scroll_bar_data_vec
    }
}
