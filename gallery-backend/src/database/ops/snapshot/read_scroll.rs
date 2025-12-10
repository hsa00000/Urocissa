use super::{TreeSnapshot, new::SNAPSHOTS_TABLE};
use crate::{
    public::db::tree_snapshot::read_tree_snapshot::MyCow,
    public::structure::reduced_data::ReducedData, public::structure::row::ScrollBarData,
};
use chrono::{Datelike, TimeZone, Utc};
use log::info;
use std::time::Instant;

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: u128) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        let tree_snapshot = self.read_tree_snapshot(&timestamp).unwrap();
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        let mut process_data = |index: usize, data: &ReducedData| {
            let datetime = Utc.timestamp_opt(data.date as i64, 0).unwrap();
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
            MyCow::Redb(txn, ts) => {
                let table = txn.open_table(SNAPSHOTS_TABLE).unwrap();
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
