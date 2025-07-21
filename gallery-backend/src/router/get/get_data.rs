use crate::operations::open_db::{open_data_and_album_tables, open_tree_snapshot_table};
use crate::operations::resolve_show_download_and_metadata;
use crate::operations::transitor::{
    abstract_data_to_database_timestamp_return, clear_abstract_data_metadata,
    hash_to_abstract_data, index_to_hash,
};
use crate::public::db::tree_snapshot::TREE_SNAPSHOT;
use crate::public::structure::database_struct::database_timestamp::DataBaseTimestampReturn;
use crate::public::structure::row::{Row, ScrollBarData};

use crate::router::AppResult;
use crate::router::fairing::guard_timestamp::GuardTimestamp;
use anyhow::{Result, anyhow};
use log::info;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rocket::serde::json::Json;
use std::time::Instant;

#[get("/get/get-data?<timestamp>&<start>&<end>")]
pub async fn get_data(
    guard_timestamp: GuardTimestamp,
    timestamp: u128,
    start: usize,
    end: usize,
) -> AppResult<Json<Vec<DataBaseTimestampReturn>>> {
    tokio::task::spawn_blocking(move || -> AppResult<Json<Vec<DataBaseTimestampReturn>>> {
        let start_time = Instant::now();

        let resolved_share_opt = guard_timestamp.claims.resolved_share_opt;
        let (show_download, show_metadata) = resolve_show_download_and_metadata(resolved_share_opt);

        let (data_table, album_table) = open_data_and_album_tables();
        let tree_snapshot = open_tree_snapshot_table(timestamp)?;

        if start >= end || end > tree_snapshot.len() {
            return Ok(Json(vec![]));
        }

        let database_timestamp_return_list: Result<Vec<DataBaseTimestampReturn>> = (start..end)
            .into_par_iter()
            .map(|index| {
                let hash = index_to_hash(&tree_snapshot, index)
                    .map_err(|e| anyhow!("Failed to read hash by index {}: {}", index, e))?;

                let mut abstract_data = hash_to_abstract_data(&data_table, &album_table, hash)
                    .map_err(|e| anyhow!("Failed to read abstract data by hash {}: {}", hash, e))?;

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
        database_timestamp_return_list.map(Json).map_err(Into::into)
    })
    .await
    .unwrap()
}

#[get("/get/get-rows?<index>&<timestamp>")]
pub async fn get_rows(
    _auth: GuardTimestamp,
    index: usize,
    timestamp: u128,
) -> AppResult<Json<Row>> {
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();
        let filtered_rows = TREE_SNAPSHOT.read_row(index, timestamp)?;
        let duration = format!("{:?}", start_time.elapsed());
        info!(duration = &*duration; "Read rows: index = {}", index);
        return Ok(Json(filtered_rows));
    })
    .await
    .unwrap()
}

#[get("/get/get-scroll-bar?<timestamp>")]
pub async fn get_scroll_bar(_auth: GuardTimestamp, timestamp: u128) -> Json<Vec<ScrollBarData>> {
    let scrollbar_data = TREE_SNAPSHOT.read_scrollbar(timestamp);
    Json(scrollbar_data)
}
