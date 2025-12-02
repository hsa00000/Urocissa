use std::time::Instant;

use super::TreeSnapshot;
use crate::{
    public::db::tree_snapshot::read_tree_snapshot::MyCow, public::structure::row::ScrollBarData,
    public::structure::reduced_data::ReducedData,
};

use chrono::{Datelike, TimeZone, Utc};

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: u128) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        let tree_snapshot = self.read_tree_snapshot(&timestamp).unwrap();
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        // 輔助閉包，用來處理資料並生成 scrollbar
        let mut process_data = |index: usize, data: &ReducedData| {
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
        };

        match tree_snapshot {
            MyCow::DashMap(ref_data) => {
                ref_data.iter().enumerate().for_each(|(index, data)| {
                    process_data(index, data);
                });
            }
            MyCow::Sqlite(pool, timestamp_str) => {
                let conn = pool.get().unwrap();
                let mut stmt = conn
                    .prepare("SELECT data FROM snapshots WHERE timestamp = ? ORDER BY row_index")
                    .unwrap();
                
                let rows = stmt
                    .query_map([timestamp_str], |row| {
                        let blob: Vec<u8> = row.get(0)?;
                        Ok(blob)
                    })
                    .unwrap();

                for (index, blob_result) in rows.enumerate() {
                    if let Ok(blob) = blob_result {
                        if let Ok(data) = bitcode::decode::<ReducedData>(&blob) {
                            process_data(index, &data);
                        }
                    }
                }
            }
        }
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Generate scrollbar");
        scroll_bar_data_vec
    }
}
