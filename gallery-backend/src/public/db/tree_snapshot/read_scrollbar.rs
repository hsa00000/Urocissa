use std::time::Instant;

use super::TreeSnapshot;
use crate::public::structure::response::row::ScrollBarData;

use chrono::{Datelike, TimeZone, Utc};

impl TreeSnapshot {
    pub fn read_scrollbar(&'static self, timestamp: i64) -> Vec<ScrollBarData> {
        let start_time = Instant::now();
        let tree_snapshot = self.read_tree_snapshot(timestamp).unwrap();
        let mut scroll_bar_data_vec = Vec::new();
        let mut last_year = None;
        let mut last_month = None;

        tree_snapshot
            .for_each_timestamp(|index, timestamp| {
                let datetime = Utc.timestamp_millis_opt(timestamp).unwrap();
                let year = datetime.year();
                let month = datetime.month();
                if last_year != Some(year) || last_month != Some(month) {
                    last_year = Some(year);
                    last_month = Some(month);
                    scroll_bar_data_vec.push(ScrollBarData {
                        #[allow(clippy::cast_sign_loss)]
                        year: year as usize,
                        #[allow(clippy::cast_sign_loss)]
                        month: month as usize,
                        index,
                    });
                }
                Ok(())
            })
            .unwrap();
        crate::perf_timing!(
            "tree_snapshot.generate_scrollbar",
            start_time,
            "Generate scrollbar"
        );
        scroll_bar_data_vec
    }
}
