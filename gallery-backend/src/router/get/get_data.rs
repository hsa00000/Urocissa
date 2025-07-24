use crate::operations::open_db::{open_data_and_album_tables, open_tree_snapshot_table};
use crate::operations::resolve_show_download_and_metadata;
use crate::operations::transitor::{
    abstract_data_to_database_timestamp_return, clear_abstract_data_metadata,
    hash_to_abstract_data, index_to_hash,
};
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::database_struct::database_timestamp::DataBaseTimestampReturn;
use crate::public::structure::row::{Row, ScrollBarData};

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
) -> AppResult<Json<Vec<DataBaseTimestampReturn>>> {
    let guard_timestamp = guard_timestamp?;
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();

        let resolved_share_opt = guard_timestamp.claims.resolved_share_opt;
        let (show_download, show_metadata) = resolve_show_download_and_metadata(resolved_share_opt);

        let (data_table, album_table) = open_data_and_album_tables();
        let tree_snapshot = open_tree_snapshot_table(timestamp)?;
        end = end.min(tree_snapshot.len());

        if start >= end {
            return Ok(Json(vec![]));
        }

        let database_timestamp_return_list: Result<_> = (start..end)
            .into_par_iter()
            .map(|index| {
                let hash = index_to_hash(&tree_snapshot, index)?;

                let mut abstract_data = hash_to_abstract_data(&data_table, &album_table, hash)?;

                clear_abstract_data_metadata(&mut abstract_data, show_metadata);

                let database_timestamp_return = abstract_data_to_database_timestamp_return(
                    abstract_data,
                    timestamp,
                    show_download,
                );
                Ok(database_timestamp_return)
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
