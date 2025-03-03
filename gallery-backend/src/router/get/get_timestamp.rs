use crate::public::abstract_data::AbstractData;
use crate::public::constant::DEFAULT_PRIORITY_LIST;
use crate::public::database_struct::database_timestamp::DataBaseTimestamp;
use crate::public::redb::{ALBUM_TABLE, DATA_TABLE};
use crate::public::row::{Row, ScrollBarData};
use crate::public::tree::TREE;
use crate::public::tree_snapshot::TREE_SNAPSHOT;

use crate::router::fairing::timestamp_guard::TimestampGuard;
use log::info;
use rocket::http::Status;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rocket::serde::json::Json;
use std::time::Instant;

#[get("/get/get-data?<timestamp>&<start>&<end>")]
pub async fn get_data(
    _auth: TimestampGuard,
    timestamp: u128,
    start: usize,
    end: usize,
) -> Json<Vec<DataBaseTimestamp>> {
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&timestamp).unwrap();
        let read_txn = TREE.in_disk.begin_read().unwrap();
        let table = read_txn.open_table(DATA_TABLE).unwrap();
        let album_table = read_txn.open_table(ALBUM_TABLE).unwrap();
        let end = end.min(tree_snapshot.len());

        // Early return if start is out of range.
        if start >= end {
            return Json(vec![]);
        }

        let data_vec: Vec<DataBaseTimestamp> = (start..end)
            .into_par_iter()
            .map(|index| {
                let hash = tree_snapshot.get_hash(index);
                if let Some(database) = table.get(&*hash).unwrap() {
                    DataBaseTimestamp::new(
                        AbstractData::Database(database.value()),
                        &DEFAULT_PRIORITY_LIST,
                    )
                } else if let Some(album) = album_table.get(&*hash).unwrap() {
                    DataBaseTimestamp::new(
                        AbstractData::Album(album.value()),
                        &DEFAULT_PRIORITY_LIST,
                    )
                } else {
                    panic!("Entry not found for hash: {:?}", hash);
                }
            })
            .collect();
        let duration = format!("{:?}", start_time.elapsed());
        info!(duration = &*duration; "Get data: {} ~ {}", start, end);
        Json(data_vec)
    })
    .await
    .unwrap()
}

#[get("/get/get-rows?<index>&<timestamp>")]
pub async fn get_rows(
    _auth: TimestampGuard,
    index: usize,
    timestamp: u128,
) -> Result<Json<Row>, Status> {
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
pub async fn get_scroll_bar(_auth: TimestampGuard, timestamp: u128) -> Json<Vec<ScrollBarData>> {
    let scrollbar_data = TREE_SNAPSHOT.read_scrollbar(timestamp);
    Json(scrollbar_data)
}
