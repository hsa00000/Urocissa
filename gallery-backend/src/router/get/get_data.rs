// src/router/get/get_data.rs

use crate::operations::open_db::{open_data_table, open_tree_snapshot_table};
use crate::operations::resolve_show_download_and_metadata;
use crate::operations::transitor::{abstract_data_to_database_timestamp_return, index_to_hash};
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::db::write_behind::WRITE_BEHIND;
use crate::public::structure::response::database_timestamp::DataBaseTimestampReturn;
use crate::public::structure::response::row::{Row, ScrollBarData};

use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::router::fairing::guard_timestamp::GuardTimestamp;
use crate::router::{AppResult, GuardResult};
use anyhow::Result;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rocket::serde::json::Json;
use std::time::Instant;

#[get("/get/get-data?<timestamp>&<start>&<end>")]
pub async fn get_data(
    guard_timestamp: GuardResult<GuardTimestamp>,
    timestamp: i64,
    start: usize,
    mut end: usize,
) -> AppResult<Json<Vec<DataBaseTimestampReturn>>> {
    let guard_timestamp = guard_timestamp?;
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();

        let resolved_share_opt = guard_timestamp.claims.resolved_share_opt;
        let (show_download, show_metadata) = resolve_show_download_and_metadata(resolved_share_opt);

        let data_table = open_data_table();
        let tree_snapshot = open_tree_snapshot_table(timestamp)
            .or_raise(|| (ErrorKind::Database, "Failed to open tree snapshot table"))?;

        end = end.min(tree_snapshot.len());

        if start >= end {
            return Ok(Json(vec![]));
        }

        let database_timestamp_return_list: Result<Vec<_>, AppError> = (start..end)
            .into_par_iter()
            .map(|index| {
                let hash = index_to_hash(&tree_snapshot, index).or_raise(|| {
                    (
                        ErrorKind::Database,
                        format!("Failed to map index {index} to hash"),
                    )
                })?;

                let durable = data_table
                    .get(hash.as_str())
                    .or_raise(|| (ErrorKind::Database, "Failed to read durable data"))?
                    .map(|value| value.into_value());
                let abstract_data = WRITE_BEHIND
                    .logical_record(hash.as_str(), durable)
                    .ok_or_else(|| {
                        AppError::new(
                            ErrorKind::NotFound,
                            format!("Failed to retrieve logical data for hash {hash}"),
                        )
                    })?;

                let database_timestamp_return = abstract_data_to_database_timestamp_return(
                    abstract_data,
                    timestamp,
                    show_download,
                    show_metadata,
                );
                Ok(database_timestamp_return)
            })
            .collect();

        crate::perf_timing!(
            "get_data.read_range",
            start_time,
            "Get data: {start} ~ {end}"
        );
        Ok(Json(database_timestamp_return_list?))
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))?
}

#[get("/get/get-rows?<index>&<timestamp>")]
pub async fn get_rows(
    auth: GuardResult<GuardTimestamp>,
    index: usize,
    timestamp: i64,
) -> AppResult<Json<Row>> {
    let _ = auth;
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();
        let filtered_rows = TREE_SNAPSHOT
            .read_row(index, timestamp)
            .or_raise(|| (ErrorKind::Database, "Failed to read row from snapshot"))?;
        crate::perf_timing!(
            "get_data.read_rows",
            start_time,
            "Read rows: index = {index}"
        );
        Ok(Json(filtered_rows))
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))?
}

#[get("/get/get-scroll-bar?<timestamp>")]
#[allow(clippy::needless_pass_by_value)]
pub fn get_scroll_bar(
    auth: GuardResult<GuardTimestamp>,
    timestamp: i64,
) -> Json<Vec<ScrollBarData>> {
    let _ = auth;
    let scrollbar_data = TREE_SNAPSHOT.read_scrollbar(timestamp);
    Json(scrollbar_data)
}
