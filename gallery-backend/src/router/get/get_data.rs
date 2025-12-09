use crate::public::db::tree::TREE;
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::abstract_data::AbstractDataResponse;
use crate::public::structure::row::{Row, ScrollBarData};
use crate::workflow::processors::transitor::{
    index_to_hash, process_abstract_data_for_response, resolve_show_download_and_metadata,
};

use crate::router::fairing::guard_timestamp::GuardTimestamp;
use crate::router::{AppResult, GuardResult};
use anyhow::Result;
use log::info;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rocket::serde::json::Json;
use std::time::Instant;

#[get("/get/get-data?<timestamp>&<start>&<end>")]
pub async fn get_data(
    guard_timestamp: GuardResult<GuardTimestamp>,
    timestamp: u128,
    start: usize,
    mut end: usize,
) -> AppResult<Json<Vec<AbstractDataResponse>>> {
    let guard_timestamp = guard_timestamp?;
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();

        let resolved_share_opt = guard_timestamp.claims.resolved_share_opt;
        let (show_download, show_metadata) = resolve_show_download_and_metadata(resolved_share_opt);

        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&timestamp).unwrap();
        end = end.min(tree_snapshot.len());

        if start >= end {
            return Ok(Json(vec![]));
        }

        let database_timestamp_return_list: Result<_> = (start..end)
            .into_par_iter()
            .map(|index| {
                let hash = index_to_hash(&tree_snapshot, index)?;

                let abstract_data = TREE.load_from_db(&hash)?;

                let processed_data =
                    process_abstract_data_for_response(abstract_data, show_metadata);
                Ok(processed_data.to_response(guard_timestamp.claims.timestamp, show_download))
            })
            .collect();

        let duration = format!("{:?}", start_time.elapsed());
        info!(duration = &*duration; "Get data: {} ~ {}", start, end);
        Ok(Json(database_timestamp_return_list?))
    })
    .await?
}

#[get("/get/get-rows?<index>&<timestamp>")]
pub async fn get_rows(
    auth: GuardResult<GuardTimestamp>,
    index: usize,
    timestamp: u128,
) -> AppResult<Json<Row>> {
    let _ = auth;
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();
        let filtered_rows = TREE_SNAPSHOT.read_row(index, timestamp)?;
        let duration = format!("{:?}", start_time.elapsed());
        info!(duration = &*duration; "Read rows: index = {}", index);
        Ok(Json(filtered_rows))
    })
    .await?
}

#[get("/get/get-scroll-bar?<timestamp>")]
pub async fn get_scroll_bar(
    auth: GuardResult<GuardTimestamp>,
    timestamp: u128,
) -> Json<Vec<ScrollBarData>> {
    let _ = auth;
    let scrollbar_data = TREE_SNAPSHOT.read_scrollbar(timestamp);
    Json(scrollbar_data)
}
