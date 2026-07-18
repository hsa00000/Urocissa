use crate::public::constant::runtime::QUERY_RAYON_POOL;
use crate::public::db::query_snapshot::QUERY_SNAPSHOT;
use crate::public::db::tree::TREE;
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::public::db::tree::state::{SlotRef, TargetSet};
use crate::public::db::tree_snapshot::read_scrollbar::build_scrollbar;
use crate::public::db::tree_snapshot::{PendingTreeSnapshot, TREE_SNAPSHOT};
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::album::ResolvedShare;
use crate::public::structure::expression::{AlbumFilterValue, Expression};
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::claims::claims_timestamp::ClaimsTimestamp;
use crate::router::fairing::guard_share::GuardShare;
use crate::tasks::BATCH_COORDINATOR;

use crate::tasks::batcher::flush_query_snapshot::FlushQuerySnapshotTask;
use crate::tasks::batcher::flush_tree_snapshot::FlushTreeSnapshotTask;

use anyhow::Result;
use bitcode::{Decode, Encode};
use chrono::Utc;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};
use std::mem;
use std::sync::atomic::Ordering;
use std::time::Instant;

const PARALLEL_FILTER_THRESHOLD: usize = 32_768;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct Prefetch {
    pub timestamp: i64,
    pub locate_to: Option<usize>,
    pub data_length: usize,
}

impl Prefetch {
    fn new(timestamp: i64, locate_to: Option<usize>, data_length: usize) -> Self {
        Self {
            timestamp,
            locate_to,
            data_length,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefetchReturn {
    pub prefetch: Prefetch,
    pub token: String,
    pub resolved_share_opt: Option<ResolvedShare>,
}

impl PrefetchReturn {
    fn new(prefetch: Prefetch, token: String, resolved_share_opt: Option<ResolvedShare>) -> Self {
        Self {
            prefetch,
            token,
            resolved_share_opt,
        }
    }
}

// -----------------------------------------------------------------------------
// ── Helper functions for each step ──────────────────────────────────────────
// -----------------------------------------------------------------------------

fn check_query_cache(
    query_hash: u64,
    resolved_share_option: &mut Option<ResolvedShare>,
) -> Option<Json<PrefetchReturn>> {
    let find_cache_start_time = Instant::now();

    // Check cache first
    if let Ok(Some(prefetch)) = QUERY_SNAPSHOT.read_query_snapshot(query_hash) {
        let snapshot_epoch = TREE_SNAPSHOT
            .read_tree_snapshot(prefetch.timestamp)
            .and_then(|snapshot| snapshot.structural_epoch());
        let current_epoch = TREE.state.read().ok().map(|state| state.structural_epoch());
        if snapshot_epoch
            .ok()
            .is_some_and(|epoch| Some(epoch) == current_epoch)
        {
            crate::perf_timing!(
                "prefetch.query_cache_lookup",
                find_cache_start_time,
                "Query cache found"
            );
            let claims = ClaimsTimestamp::new(mem::take(resolved_share_option), prefetch.timestamp);
            return Some(Json(PrefetchReturn::new(
                prefetch,
                claims.encode(),
                claims.resolved_share_opt,
            )));
        }
        log::warn!(
            "ignoring query cache entry {} because its compact tree snapshot is stale or corrupt",
            prefetch.timestamp
        );
    }

    crate::perf_timing!(
        "prefetch.query_cache_lookup",
        find_cache_start_time,
        "Query cache not found. Generate a new one."
    );
    None
}

fn filter_items(
    expression_option: Option<Expression>,
    resolved_share_option: Option<&ResolvedShare>,
    locate_option: Option<&String>,
) -> Result<(PendingTreeSnapshot, Option<usize>), AppError> {
    let filter_items_start_time = Instant::now();

    let tree_guard = TREE.state.read().map_err(|err| {
        AppError::new(
            ErrorKind::Internal,
            format!("Failed to read tree in memory: {err:?}"),
        )
    })?;
    let hidden_album = resolved_share_option
        .filter(|share| !share.share.show_metadata)
        .map(|share| share.album_id);
    let compile_start = Instant::now();
    let compiled_expression = expression_option
        .as_ref()
        .map(|expression| tree_guard.compile_expression(expression, hidden_album));
    crate::perf_timing!(
        "prefetch.compile_expression",
        compile_start,
        "Compile query expression"
    );

    let reduce = |slot_ref: &SlotRef| {
        let record = tree_guard.get(*slot_ref)?;
        compiled_expression
            .as_ref()
            .is_none_or(|expression| expression.matches(record, slot_ref.index()))
            .then_some((*slot_ref, record.timestamp))
    };
    let matches: Vec<(SlotRef, i64)> = if tree_guard.order.len() >= PARALLEL_FILTER_THRESHOLD {
        QUERY_RAYON_POOL.install(|| tree_guard.order.par_iter().filter_map(reduce).collect())
    } else {
        tree_guard.order.iter().filter_map(reduce).collect()
    };

    crate::perf_timing!(
        "prefetch.filter_items",
        filter_items_start_time,
        "Filter items"
    );

    let layout_start_time = Instant::now();
    let locate_slot = locate_option.and_then(|hash| tree_guard.find(hash));
    let locate_to_index = locate_slot.and_then(|target| {
        if matches.len() >= PARALLEL_FILTER_THRESHOLD {
            QUERY_RAYON_POOL.install(|| {
                matches
                    .par_iter()
                    .position_first(|(slot_ref, _)| *slot_ref == target)
            })
        } else {
            matches.iter().position(|(slot_ref, _)| *slot_ref == target)
        }
    });

    crate::perf_timing!(
        "prefetch.compute_layout",
        layout_start_time,
        "Compute layout"
    );

    let snapshot_start = Instant::now();
    // Keep the timestamp captured during matching. Re-reading the arena here
    // follows display order rather than slot order and becomes a cache-miss
    // heavy second random scan at one million records.
    let scrollbar = build_scrollbar(matches.iter().map(|(_, timestamp)| *timestamp));
    let universe = tree_guard.arena.capacity();
    let targets =
        TargetSet::from_unique_slot_refs(matches.iter().map(|(slot_ref, _)| *slot_ref), universe);
    let ordinals = matches
        .into_iter()
        .map(|(slot_ref, _)| slot_ref.index())
        .collect();
    crate::perf_timing!(
        "prefetch.build_compact_snapshot",
        snapshot_start,
        "Build compact snapshot order and target bitmap"
    );
    Ok((
        PendingTreeSnapshot {
            structural_epoch: tree_guard.structural_epoch(),
            universe,
            ordinals,
            targets,
            scrollbar,
        },
        locate_to_index,
    ))
}

fn build_cache_key(expression_option: Option<&Expression>, locate_option: Option<&String>) -> u64 {
    let cache_key_start_time = Instant::now();

    let mut hasher = DefaultHasher::new();
    expression_option.hash(&mut hasher);
    VERSION_COUNT_TIMESTAMP
        .load(Ordering::Relaxed)
        .hash(&mut hasher);
    locate_option.hash(&mut hasher);
    let query_hash = hasher.finish();

    crate::perf_timing!(
        "prefetch.build_cache_key",
        cache_key_start_time,
        "Build cache key"
    );

    query_hash
}

fn insert_data_into_tree_snapshot(snapshot: PendingTreeSnapshot) -> (i64, usize) {
    let db_start_time = Instant::now();

    // Persist to snapshot
    let timestamp_millis = Utc::now().timestamp_millis();
    let snapshot_length = snapshot.ordinals.len();
    TREE_SNAPSHOT.in_memory.insert(timestamp_millis, snapshot);
    BATCH_COORDINATOR.execute_batch_detached(FlushTreeSnapshotTask);

    crate::perf_timing!(
        "prefetch.write_snapshot_memory",
        db_start_time,
        "Write cache into memory"
    );

    (timestamp_millis, snapshot_length)
}

fn create_json_response(
    timestamp_millis: i64,
    locate_to_index: Option<usize>,
    reduced_data_vector_length: usize,
    query_hash: u64,
    resolved_share_option: Option<ResolvedShare>,
) -> Json<PrefetchReturn> {
    let json_start_time = Instant::now();

    let prefetch = Prefetch::new(
        timestamp_millis,
        locate_to_index,
        reduced_data_vector_length,
    );

    // Cache the result
    QUERY_SNAPSHOT.in_memory.insert(query_hash, prefetch);
    BATCH_COORDINATOR.execute_batch_detached(FlushQuerySnapshotTask);

    // Build response
    let claims = ClaimsTimestamp::new(resolved_share_option, timestamp_millis);
    let json = Json(PrefetchReturn::new(
        prefetch,
        claims.encode(),
        claims.resolved_share_opt,
    ));

    crate::perf_timing!(
        "prefetch.create_json",
        json_start_time,
        "Create JSON response"
    );

    json
}

// -----------------------------------------------------------------------------
// ── Single prefetch function ─────────────────────────────────────────────────
// -----------------------------------------------------------------------------

fn execute_prefetch_logic(
    expression_option: Option<Expression>,
    locate_option: Option<&String>,
    mut resolved_share_option: Option<ResolvedShare>,
) -> Result<Json<PrefetchReturn>, AppError> {
    // Start timer
    let start_time = Instant::now();

    // Step 1: Build cache key for response creation
    let query_hash = build_cache_key(expression_option.as_ref(), locate_option);

    // Step 2: Check if query cache is available
    if let Some(cached_response) = check_query_cache(query_hash, &mut resolved_share_option) {
        return Ok(cached_response);
    }

    // Step 3: Filter items
    let (snapshot, locate_to_index) = filter_items(
        expression_option,
        resolved_share_option.as_ref(),
        locate_option,
    )?;

    // Step 6: Insert data into TREE_SNAPSHOT
    let (timestamp_millis, reduced_data_vector_length) = insert_data_into_tree_snapshot(snapshot);

    // Step 7: Create and return JSON response
    let json = create_json_response(
        timestamp_millis,
        locate_to_index,
        reduced_data_vector_length,
        query_hash,
        resolved_share_option,
    );

    // Total elapsed time
    crate::perf_timing!(
        "prefetch.total",
        start_time,
        "(total time) Get_data_length complete"
    );

    Ok(json)
}

#[post("/get/prefetch?<locate>", format = "json", data = "<query_data>")]
pub async fn prefetch(
    auth_guard: GuardResult<GuardShare>,
    query_data: Option<Json<Expression>>,
    locate: Option<String>,
) -> AppResult<Json<PrefetchReturn>> {
    let auth_guard = auth_guard?;
    // Combine album filter (if any) with the client‑supplied query.
    let mut combined_expression_option = query_data.map(rocket::serde::json::Json::into_inner);
    let resolved_share_option = auth_guard.claims.get_share();

    if let Some(resolved_share) = &resolved_share_option {
        let album_filter_expression =
            Expression::Album(AlbumFilterValue::Value(resolved_share.album_id));

        combined_expression_option = Some(match combined_expression_option {
            Some(client_expression) => {
                Expression::And(vec![album_filter_expression, client_expression])
            }
            None => album_filter_expression,
        });
    }

    // Execute on blocking thread
    let job_handle = tokio::task::spawn_blocking(move || {
        execute_prefetch_logic(
            combined_expression_option,
            locate.as_ref(),
            resolved_share_option,
        )
    })
    .await
    .or_raise(|| (ErrorKind::Internal, "Failed to join blocking task"))??;

    Ok(job_handle)
}
