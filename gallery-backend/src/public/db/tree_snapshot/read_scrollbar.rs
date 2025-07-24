use std::time::Instant;

use super::TreeSnapshot;
use crate::{
    public::db::tree_snapshot::read_tree_snapshot::MyCow, public::structure::row::ScrollBarData,
};

use chrono::{Datelike, TimeZone, Utc};
use redb::ReadableTable;

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: u128) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        let tree_snapshot = self.read_tree_snapshot(&timestamp).unwrap();
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        match tree_snapshot {
            MyCow::DashMap(ref_data) => {
                ref_data.iter().enumerate().for_each(|(index, data)| {
                    let datetime = Utc.timestamp_millis_opt(data.date as i64).unwrap();
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
                });
            }
            MyCow::Redb(redb) => {
                redb.iter()
                    .unwrap()
                    .enumerate()
                    .for_each(|(index, result)| {
                        let (_key, value) = result.unwrap();
                        let data = value.value();
                        let datetime = Utc.timestamp_millis_opt(data.date as i64).unwrap();
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
                    });
            }
        }
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Generate scrollbar");
        scroll_bar_data_vec
    }
}
