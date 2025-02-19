use crate::public::abstract_data::AbstractData;
use crate::public::constant::DEFAULT_PRIORITY_LIST;
use crate::public::database_struct::database_timestamp::DataBaseTimestamp;
use crate::public::redb::{ALBUM_TABLE, DATA_TABLE};
use crate::public::row::{Row, ScrollBarData};
use crate::public::tree::TREE;
use crate::public::tree_snapshot::TREE_SNAPSHOT;
use crate::router::fairing::AuthGuard;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rocket::http::Status;
use rocket::serde::json::Json;
use std::time::Instant;

#[get("/get/get-data?<timestamp>&<start>&<end>")]
pub async fn get_data(
    _auth: AuthGuard,
    timestamp: u128,
    start: usize,
    end: usize,
) -> Result<Json<Vec<DataBaseTimestamp>>, Status> {
    tokio::task::spawn_blocking(move || {
        let start_time = Instant::now();
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&timestamp).unwrap();
        let read_txn = TREE.in_disk.begin_read().unwrap();
        let table = read_txn.open_table(DATA_TABLE).unwrap();
        let album_table = read_txn.open_table(ALBUM_TABLE).unwrap();
        let end = end.min(tree_snapshot.len());

        if start < end {
            // Change the type of data_vec to Result<Vec<DataBaseTimestamp>, Status>
            let data_vec: Result<Vec<DataBaseTimestamp>, Status> = (start..end)
                .into_par_iter()
                .map(
                    |index| match table.get(&*tree_snapshot.get_hash(index)).unwrap() {
                        Some(database) => Ok(DataBaseTimestamp::new(
                            AbstractData::Database(database.value()),
                            &DEFAULT_PRIORITY_LIST,
                        )),
                        None => match album_table.get(&*tree_snapshot.get_hash(index)).unwrap() {
                            Some(album) => Ok(DataBaseTimestamp::new(
                                AbstractData::Album(album.value()),
                                &DEFAULT_PRIORITY_LIST,
                            )),
                            None => Err(Status::InternalServerError),
                        },
                    },
                )
                .collect();

            match data_vec {
                Ok(vec) => {
                    let duration = format!("{:?}", start_time.elapsed());
                    info!(duration = &*duration; "Get data: {} ~ {}", start, end);

                    Ok(Json(vec))
                }
                Err(e) => Err(e),
            }
        } else {
            // Index out of range
            Ok(Json(vec![]))
        }
    })
    .await
    .unwrap()
}

#[get("/get/get-rows?<index>&<timestamp>")]
pub async fn get_rows(
    _auth: AuthGuard,
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
pub async fn get_scroll_bar(
    _auth: AuthGuard,
    timestamp: u128,
) -> Result<Json<Vec<ScrollBarData>>, Status> {
    let scrollbar_data = TREE_SNAPSHOT.read_scrollbar(timestamp);
    Ok(Json(scrollbar_data))
}
