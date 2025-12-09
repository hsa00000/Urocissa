use std::time::Instant;

use super::{TreeSnapshot, new::SNAPSHOTS_TABLE};
use crate::{
    public::db::tree_snapshot::read_tree_snapshot::MyCow,
    public::structure::reduced_data::ReducedData, public::structure::row::ScrollBarData,
};

use chrono::{Datelike, TimeZone, Utc};
use log::info;
use redb::ReadableTable;

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: u128) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        let tree_snapshot = self.read_tree_snapshot(&timestamp).unwrap();
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        // 輔助閉包
        let mut process_data = |index: usize, data: &ReducedData| {
            // 注意：這裡假設 data.date 是 Unix Timestamp (Seconds)
            // 如果是 Millis，請改用 timestamp_millis_opt
            let datetime = Utc.timestamp_opt(data.date as i64, 0).unwrap();
            let year = datetime.year();
            let month = datetime.month();
            if last_year != Some(year) || last_month != Some(month) {
                last_year = Some(year);
                last_month = Some(month);
                let scrollbar_data = ScrollBarData {
                    year: year as usize,
                    month: month as usize,
                    index: index,
                };
                scroll_bar_data_vec.push(scrollbar_data)
            }
        };

        match tree_snapshot {
            MyCow::DashMap(ref_data) => {
                ref_data.iter().enumerate().for_each(|(index, data)| {
                    process_data(index, data);
                });
            }
            MyCow::Redb(txn, ts) => {
                let table = txn.open_table(SNAPSHOTS_TABLE).unwrap();
                let start = (ts, 0);
                let end = (ts, u64::MAX);

                // Range Scan 會自動依照 Key 排序，Key 是 (Timestamp, RowIndex)
                // 所以遍歷出來的順序就是 RowIndex 的順序，無需手動 sort
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
