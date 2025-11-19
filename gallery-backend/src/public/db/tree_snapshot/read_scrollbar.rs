use std::time::Instant;

use super::TreeSnapshot;
use crate::{
    public::structure::row::ScrollBarData,
    public::db::sqlite::SQLITE,
};

use chrono::{Datelike, TimeZone, Utc};

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: u128) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        // We don't strictly need read_tree_snapshot here if we just query SQLite directly
        // But to keep consistency, we can use it or just query SQLITE.
        // Since read_tree_snapshot returns a reader that wraps timestamp, we can use the timestamp.
        
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        if let Ok(dates) = SQLITE.get_snapshot_dates(timestamp) {
            for (index, date_val) in dates {
                let datetime = Utc.timestamp_millis_opt(date_val).unwrap();
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
            }
        }
        
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Generate scrollbar");
        scroll_bar_data_vec
    }
}
