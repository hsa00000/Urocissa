use crate::public::constant::runtime::QUERY_RAYON_POOL;
use crate::public::db::query_snapshot::QUERY_SNAPSHOT;
use crate::public::db::tree::TREE;
use crate::public::db::tree::VERSION_COUNT_TIMESTAMP;
use crate::public::db::tree::state::{SlotRef, TargetSet, TargetSetBuilder};
use crate::public::db::tree_snapshot::read_scrollbar::timestamp_year_month;
use crate::public::db::tree_snapshot::{PendingTreeSnapshot, TREE_SNAPSHOT};
use crate::public::error::{AppError, ErrorKind, ResultExt};
use crate::public::structure::album::ResolvedShare;
use crate::public::structure::expression::{AlbumFilterValue, Expression};
use crate::public::structure::response::row::ScrollBarData;
use crate::router::AppResult;
use crate::router::GuardResult;
use crate::router::claims::claims_timestamp::ClaimsTimestamp;
use crate::router::fairing::guard_share::GuardShare;
use crate::tasks::BATCH_COORDINATOR;

use crate::tasks::batcher::flush_query_snapshot::FlushQuerySnapshotTask;
use crate::tasks::batcher::flush_tree_snapshot::FlushTreeSnapshotTask;

use anyhow::Result;
use bitcode::{Decode, Encode};
use chrono::{TimeZone, Utc};
use rayon::iter::ParallelIterator;
use rayon::slice::ParallelSlice;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};
use std::mem;
use std::sync::atomic::Ordering;
use std::time::Instant;

const PARALLEL_FILTER_THRESHOLD: usize = 32_768;
const PARALLEL_FILTER_CHUNK_ITEMS: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompactScrollBoundary {
    local_index: u32,
    year_month: i32,
}

#[derive(Debug, Clone, Copy)]
struct TimestampMonthRange {
    start: i64,
    end: i64,
    year_month: i32,
}

impl TimestampMonthRange {
    fn for_timestamp(timestamp: i64) -> Self {
        let (year, month) = timestamp_year_month(timestamp);
        let start = Utc
            .with_ymd_and_hms(year, month, 1, 0, 0, 0)
            .single()
            .expect("record month must be representable")
            .timestamp_millis();
        let (next_year, next_month) = if month == 12 {
            (year.checked_add(1), 1)
        } else {
            (Some(year), month + 1)
        };
        let end = next_year
            .and_then(|next_year| {
                Utc.with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
                    .single()
            })
            .map_or(i64::MAX, |datetime| datetime.timestamp_millis());
        let month = i32::try_from(month).expect("record month must fit in i32");
        let year_month = year
            .checked_mul(16)
            .and_then(|value| value.checked_add(month))
            .expect("record year-month must fit in i32");
        Self {
            start,
            end,
            year_month,
        }
    }

    fn contains(self, timestamp: i64) -> bool {
        timestamp >= self.start && timestamp < self.end
    }
}

#[derive(Debug, Default)]
struct PrefetchMatchChunk {
    ordinals: Vec<u32>,
    generation_overrides: Vec<(u32, u32)>,
    scrollbar: Vec<CompactScrollBoundary>,
    locate_to: Option<u32>,
    timestamp_month: Option<TimestampMonthRange>,
}

impl PrefetchMatchChunk {
    fn push(&mut self, slot_ref: SlotRef, timestamp: i64, is_locate: bool) {
        let local_index =
            u32::try_from(self.ordinals.len()).expect("prefetch chunk exceeds u32 indices");
        self.ordinals.push(slot_ref.index());
        if slot_ref.generation() != 1 {
            self.generation_overrides
                .push((local_index, slot_ref.generation()));
        }

        let timestamp_month = self
            .timestamp_month
            .filter(|month| month.contains(timestamp))
            .unwrap_or_else(|| TimestampMonthRange::for_timestamp(timestamp));
        self.timestamp_month = Some(timestamp_month);
        let year_month = timestamp_month.year_month;
        if self
            .scrollbar
            .last()
            .is_none_or(|boundary| boundary.year_month != year_month)
        {
            self.scrollbar.push(CompactScrollBoundary {
                local_index,
                year_month,
            });
        }
        if is_locate && self.locate_to.is_none() {
            self.locate_to = Some(local_index);
        }
    }

    fn len(&self) -> usize {
        self.ordinals.len()
    }

    #[cfg(test)]
    fn estimated_bytes(&self) -> usize {
        self.ordinals.capacity() * std::mem::size_of::<u32>()
            + self.generation_overrides.capacity() * std::mem::size_of::<(u32, u32)>()
            + self.scrollbar.capacity() * std::mem::size_of::<CompactScrollBoundary>()
    }
}

fn summarize_match_chunks(chunks: &[PrefetchMatchChunk]) -> (usize, Option<usize>) {
    let mut total = 0;
    let mut locate_to = None;
    for chunk in chunks {
        if locate_to.is_none()
            && let Some(local_index) = chunk.locate_to
        {
            locate_to = Some(total + local_index as usize);
        }
        total += chunk.len();
    }
    (total, locate_to)
}

fn merge_match_chunks(
    chunks: Vec<PrefetchMatchChunk>,
    total: usize,
    universe: usize,
) -> Result<(Vec<u32>, TargetSet, Vec<ScrollBarData>), AppError> {
    let scrollbar_capacity = chunks
        .iter()
        .map(|chunk| chunk.scrollbar.len())
        .sum::<usize>();
    let mut ordinals = Vec::with_capacity(total);
    let mut targets = TargetSetBuilder::default();
    let mut scrollbar = Vec::with_capacity(scrollbar_capacity);
    let mut global_offset = 0;
    let mut last_year_month = None;

    for chunk in chunks {
        for boundary in chunk.scrollbar {
            if last_year_month == Some(boundary.year_month) {
                continue;
            }
            last_year_month = Some(boundary.year_month);
            let year = boundary.year_month.div_euclid(16);
            let month = boundary.year_month.rem_euclid(16) as usize;
            scrollbar.push(ScrollBarData {
                #[allow(clippy::cast_sign_loss)]
                year: year as usize,
                month,
                index: global_offset + boundary.local_index as usize,
            });
        }

        let mut generation_overrides = chunk.generation_overrides.into_iter().peekable();
        for (local_index, ordinal) in chunk.ordinals.into_iter().enumerate() {
            let local_index = u32::try_from(local_index)
                .map_err(|_| AppError::new(ErrorKind::Internal, "prefetch chunk is too large"))?;
            let generation = if generation_overrides
                .peek()
                .is_some_and(|(override_index, _)| *override_index == local_index)
            {
                generation_overrides
                    .next()
                    .map_or(1, |(_, generation)| generation)
            } else {
                1
            };
            if !targets.insert(SlotRef::new(ordinal, generation)) {
                return Err(AppError::new(
                    ErrorKind::Internal,
                    "tree order contains a duplicate slot ordinal",
                ));
            }
            ordinals.push(ordinal);
        }
        if generation_overrides.next().is_some() {
            return Err(AppError::new(
                ErrorKind::Internal,
                "prefetch generation override is out of range",
            ));
        }
        global_offset = ordinals.len();
    }

    if ordinals.len() != total {
        return Err(AppError::new(
            ErrorKind::Internal,
            "prefetch chunk cardinality changed during merge",
        ));
    }
    Ok((ordinals, targets.finish(universe), scrollbar))
}

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

    let locate_slot = locate_option.and_then(|hash| tree_guard.find(hash));
    let collect_chunk = |slot_refs: &[SlotRef]| {
        let mut chunk = PrefetchMatchChunk::default();
        for slot_ref in slot_refs {
            let Some(record) = tree_guard.get(*slot_ref) else {
                continue;
            };
            if compiled_expression
                .as_ref()
                .is_some_and(|expression| !expression.matches(record, slot_ref.index()))
            {
                continue;
            }
            chunk.push(
                *slot_ref,
                record.timestamp,
                locate_slot.is_some_and(|target| target == *slot_ref),
            );
        }
        chunk
    };
    let chunks = if tree_guard.order.len() >= PARALLEL_FILTER_THRESHOLD {
        QUERY_RAYON_POOL.install(|| {
            tree_guard
                .order
                .par_chunks(PARALLEL_FILTER_CHUNK_ITEMS)
                .map(collect_chunk)
                .collect::<Vec<_>>()
        })
    } else {
        vec![collect_chunk(tree_guard.order.as_slice())]
    };

    crate::perf_timing!(
        "prefetch.filter_items",
        filter_items_start_time,
        "Filter items"
    );

    let layout_start_time = Instant::now();
    let (match_count, locate_to_index) = summarize_match_chunks(&chunks);

    crate::perf_timing!(
        "prefetch.compute_layout",
        layout_start_time,
        "Compute layout"
    );

    let snapshot_start = Instant::now();
    let universe = tree_guard.arena.capacity();
    let (ordinals, targets, scrollbar) = merge_match_chunks(chunks, match_count, universe)?;
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

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{
        PARALLEL_FILTER_CHUNK_ITEMS, PrefetchMatchChunk, merge_match_chunks, summarize_match_chunks,
    };
    use crate::public::db::tree::state::SlotRef;
    use crate::public::db::tree_snapshot::read_scrollbar::build_scrollbar;
    use crate::public::db::tree_snapshot::{PendingTreeSnapshot, SnapshotBlobView};
    use crate::public::structure::response::row::ScrollBarData;

    fn timestamp(year: i32, month: u32, day: u32) -> i64 {
        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    fn chunk(entries: &[(SlotRef, i64, bool)]) -> PrefetchMatchChunk {
        let mut chunk = PrefetchMatchChunk::default();
        for (slot_ref, timestamp, locate) in entries {
            chunk.push(*slot_ref, *timestamp, *locate);
        }
        chunk
    }

    #[test]
    fn ordered_chunks_preserve_scrollbar_locate_and_generations() {
        let january = timestamp(2026, 1, 10);
        let february = timestamp(2026, 2, 10);
        let chunks = vec![
            chunk(&[
                (SlotRef::new(65, 1), january, false),
                (SlotRef::new(7, 2), january, true),
            ]),
            PrefetchMatchChunk::default(),
            chunk(&[
                (SlotRef::new(1, 1), january, false),
                (SlotRef::new(9, 3), february, false),
                (SlotRef::new(12, 1), january, false),
            ]),
        ];

        let (total, locate_to) = summarize_match_chunks(&chunks);
        assert_eq!(total, 5);
        assert_eq!(locate_to, Some(1));

        let (ordinals, targets, scrollbar) = merge_match_chunks(chunks, total, 128).unwrap();
        assert_eq!(ordinals, vec![65, 7, 1, 9, 12]);
        assert_eq!(targets.slot_ref_for_ordinal(7), Some(SlotRef::new(7, 2)));
        assert_eq!(targets.slot_ref_for_ordinal(9), Some(SlotRef::new(9, 3)));
        assert_eq!(
            scrollbar,
            vec![
                ScrollBarData {
                    year: 2026,
                    month: 1,
                    index: 0,
                },
                ScrollBarData {
                    year: 2026,
                    month: 2,
                    index: 3,
                },
                ScrollBarData {
                    year: 2026,
                    month: 1,
                    index: 4,
                },
            ]
        );

        let snapshot = PendingTreeSnapshot {
            structural_epoch: 41,
            universe: 128,
            ordinals,
            targets,
            scrollbar,
        };
        let bytes = snapshot.encode().unwrap();
        let view = SnapshotBlobView::new(&bytes).unwrap();
        assert_eq!(view.slot_ref(0).unwrap(), SlotRef::new(65, 1));
        assert_eq!(view.slot_ref(1).unwrap(), SlotRef::new(7, 2));
        assert_eq!(view.slot_ref(3).unwrap(), SlotRef::new(9, 3));
        assert_eq!(view.target_set().unwrap(), snapshot.targets);
    }

    #[test]
    fn single_and_multi_chunk_scrollbars_match_the_reference_builder() {
        let timestamps = vec![
            timestamp(2026, 12, 31),
            timestamp(2026, 12, 1),
            timestamp(2026, 11, 1),
            timestamp(2025, 12, 1),
            timestamp(2025, 12, 2),
            timestamp(2026, 12, 3),
        ];
        let entries = timestamps
            .iter()
            .enumerate()
            .map(|(index, timestamp)| {
                (
                    SlotRef::new(u32::try_from(index).unwrap(), 1),
                    *timestamp,
                    false,
                )
            })
            .collect::<Vec<_>>();
        let single = vec![chunk(&entries)];
        let split = vec![
            chunk(&entries[..2]),
            PrefetchMatchChunk::default(),
            chunk(&entries[2..4]),
            chunk(&entries[4..]),
        ];

        let (single_total, single_locate) = summarize_match_chunks(&single);
        let (split_total, split_locate) = summarize_match_chunks(&split);
        let single = merge_match_chunks(single, single_total, entries.len()).unwrap();
        let split = merge_match_chunks(split, split_total, entries.len()).unwrap();

        assert_eq!(single_locate, None);
        assert_eq!(split_locate, None);
        assert_eq!(single, split);
        assert_eq!(single.2, build_scrollbar(timestamps));
    }

    #[test]
    fn empty_chunks_produce_an_empty_snapshot() {
        let chunks = vec![PrefetchMatchChunk::default(), PrefetchMatchChunk::default()];
        let (total, locate_to) = summarize_match_chunks(&chunks);
        let (ordinals, targets, scrollbar) = merge_match_chunks(chunks, total, 64).unwrap();

        assert_eq!(total, 0);
        assert_eq!(locate_to, None);
        assert!(ordinals.is_empty());
        assert!(targets.is_empty());
        assert!(scrollbar.is_empty());
    }

    #[test]
    fn locate_offsets_cover_first_chunk_seams_last_and_filtered_out() {
        let date = timestamp(2026, 7, 1);
        let slot = |ordinal| SlotRef::new(ordinal, 1);

        let first = vec![chunk(&[(slot(1), date, true), (slot(2), date, false)])];
        assert_eq!(summarize_match_chunks(&first).1, Some(0));

        let seam = vec![
            chunk(&[(slot(1), date, false), (slot(2), date, false)]),
            PrefetchMatchChunk::default(),
            chunk(&[(slot(3), date, true), (slot(4), date, false)]),
        ];
        assert_eq!(summarize_match_chunks(&seam).1, Some(2));

        let last = vec![
            chunk(&[(slot(1), date, false)]),
            chunk(&[(slot(2), date, false), (slot(3), date, true)]),
        ];
        assert_eq!(summarize_match_chunks(&last).1, Some(2));

        let filtered_out = vec![chunk(&[(slot(1), date, false)])];
        assert_eq!(summarize_match_chunks(&filtered_out).1, None);
    }

    #[test]
    fn million_matches_stay_within_the_prefetch_working_memory_gate() {
        const ITEM_COUNT: u32 = 1_000_000;
        const MEMORY_GATE_BYTES: usize = 9 * 1024 * 1024;

        assert_eq!(std::mem::size_of::<super::CompactScrollBoundary>(), 8);
        let same_month = timestamp(2026, 7, 1);
        let chunk_items = u32::try_from(PARALLEL_FILTER_CHUNK_ITEMS).unwrap();
        let mut chunks = Vec::new();
        for start in (0..ITEM_COUNT).step_by(PARALLEL_FILTER_CHUNK_ITEMS) {
            let end = start.saturating_add(chunk_items).min(ITEM_COUNT);
            let mut current = PrefetchMatchChunk::default();
            for ordinal in start..end {
                current.push(SlotRef::new(ordinal, 1), same_month, false);
            }
            chunks.push(current);
        }
        let input_bytes = chunks.capacity() * std::mem::size_of::<PrefetchMatchChunk>()
            + chunks
                .iter()
                .map(PrefetchMatchChunk::estimated_bytes)
                .sum::<usize>();
        let (total, locate_to) = summarize_match_chunks(&chunks);
        let (ordinals, targets, scrollbar) =
            merge_match_chunks(chunks, total, ITEM_COUNT as usize).unwrap();
        let output_bytes = ordinals.capacity() * std::mem::size_of::<u32>()
            + targets.estimated_bytes()
            + scrollbar.capacity() * std::mem::size_of::<ScrollBarData>();

        assert_eq!(total, ITEM_COUNT as usize);
        assert_eq!(locate_to, None);
        assert_eq!(targets.len(), ITEM_COUNT as usize);
        assert_eq!(scrollbar.len(), 1);
        assert!(input_bytes + output_bytes <= MEMORY_GATE_BYTES);
    }
}
