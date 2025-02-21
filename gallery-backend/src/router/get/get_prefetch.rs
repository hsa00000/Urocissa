use crate::public::expression::Expression;
use crate::public::query_snapshot::QUERY_SNAPSHOT;
use crate::public::reduced_data::ReducedData;
use crate::public::tree::TREE;
use crate::public::tree::start_loop::VERSION_COUNT_TIMESTAMP;
use crate::public::tree_snapshot::TREE_SNAPSHOT;
use crate::router::fairing::AuthGuard;

use bitcode::{Decode, Encode};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};
use std::sync::atomic::Ordering;
use std::time::UNIX_EPOCH;
use std::time::{Instant, SystemTime};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Decode, Encode)]
#[serde(rename_all = "camelCase")]
pub struct Prefetch {
    pub timestamp: u128,
    pub locate_to: Option<usize>,
    pub data_length: usize,
}

impl Prefetch {
    fn new(timestamp: u128, locate_to: Option<usize>, data_length: usize) -> Self {
        Self {
            timestamp,
            locate_to,
            data_length,
        }
    }
}
#[post("/get/prefetch?<locate>", format = "json", data = "<query_data>")]
pub async fn prefetch(
    auth: AuthGuard,
    query_data: Option<Json<Expression>>,
    locate: Option<String>,
) -> Json<Option<Prefetch>> {
    tokio::task::spawn_blocking(move || {
        // Start timer
        let start_time = Instant::now();

        // Step 1: Check if query cache is available
        let find_cache_start_time = Instant::now();

        let mut expression_opt = query_data.map(|query| query.into_inner());

        if let Some(album_id) = auth.claims.album_id {
            let expression_1 = Expression::Album(album_id);
            match expression_opt {
                Some(expression) => {
                    let expression_2 = expression;
                    expression_opt = Some(Expression::And(vec![expression_1, expression_2]));
                }
                None => expression_opt = Some(expression_1),
            }
        };

        let hasher = &mut DefaultHasher::new();

        expression_opt.hash(hasher);
        VERSION_COUNT_TIMESTAMP.load(Ordering::Relaxed).hash(hasher);
        locate.hash(hasher);

        let expression_hashed = hasher.finish();

        match QUERY_SNAPSHOT.read_query_snapshot(expression_hashed) {
            Ok(Some(prefetch_opt)) => {
                let duration = format!("{:?}", find_cache_start_time.elapsed());
                info!(duration = &*duration; "Query cache found");
                return Json(prefetch_opt);
            }
            _ => {
                let duration = format!("{:?}", find_cache_start_time.elapsed());
                info!(duration = &*duration; "Query cache not found. Generate a new one.");
            }
        }

        // Step 2: Filter items
        let filter_items_start_time = Instant::now();
        let ref_data = TREE.in_memory.read().unwrap();
        let duration = format!("{:?}", filter_items_start_time.elapsed());
        info!(duration = &*duration; "Filter items");

        // Step 3: Compute layout
        let layout_start_time = Instant::now();

        let reduced_data: Vec<ReducedData> = match expression_opt {
            Some(expression) => {
                let filter = expression.generate_filter();
                ref_data
                    .par_iter()
                    .filter(move |database_timestamp| filter(&database_timestamp.abstract_data))
                    .map(|database_timestamp| ReducedData {
                        hash: database_timestamp.abstract_data.hash(),
                        width: database_timestamp.abstract_data.width(),
                        height: database_timestamp.abstract_data.height(),
                        date: database_timestamp.timestamp,
                    })
                    .collect()
            }
            None => ref_data
                .par_iter()
                .map(|database_timestamp| ReducedData {
                    hash: database_timestamp.abstract_data.hash(),
                    width: database_timestamp.abstract_data.width(),
                    height: database_timestamp.abstract_data.height(),
                    date: database_timestamp.timestamp,
                })
                .collect(),
        };

        let data_length = reduced_data.len();
        let duration = format!("{:?}", layout_start_time.elapsed());
        info!(duration = &*duration;  "Compute layout");

        // Step 4: Locate hash
        let locate_start_time = Instant::now();
        let locate_to = if let Some(ref locate_hash) = locate {
            reduced_data
                .par_iter()
                .position_first(|data| data.hash.as_str() == locate_hash)
        } else {
            None
        };

        let duration = format!("{:?}", locate_start_time.elapsed());
        info!(duration = &*duration;  "Locate data");

        // Step 5: Insert data into TREE_SNAPSHOT
        let db_start_time = Instant::now();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        TREE_SNAPSHOT.in_memory.insert(timestamp, reduced_data);
        TREE_SNAPSHOT.tree_snapshot_flush();

        let duration = format!("{:?}", db_start_time.elapsed());
        info!(duration = &*duration;   "Write cache into memory");

        // Step 6: Create and return JSON response
        let json_start_time = Instant::now();

        let prefetch_opt = Some(Prefetch::new(timestamp, locate_to, data_length));

        QUERY_SNAPSHOT
            .in_memory
            .insert(expression_hashed, prefetch_opt);
        QUERY_SNAPSHOT.query_snapshot_flush();
        let json = Json(prefetch_opt);

        let duration = format!("{:?}", json_start_time.elapsed());
        info!(duration = &*duration; "Create JSON response");

        // Total elapsed time
        let duration = format!("{:?}", start_time.elapsed());
        info!(duration = &*duration; "(total time) Get_data_length complete");

        json
    })
    .await
    .unwrap()
}
