use rocket::Route;

#[cfg(feature = "performance-test")]
mod enabled {
    use crate::operations::open_db::open_data_table;
    use crate::performance;
    use crate::public::constant::storage::get_data_path;
    use crate::public::db::{
        expire::EXPIRE,
        query_snapshot::QUERY_SNAPSHOT,
        tree::{
            TREE, VERSION_COUNT_TIMESTAMP,
            state::{TargetSet, TreeMemoryUsage},
        },
        tree_snapshot::TREE_SNAPSHOT,
        write_behind::{DirtyOperation, FLUSH_CHUNK_SIZE, WRITE_BEHIND},
    };
    use crate::public::error::{AppError, ErrorKind};
    use crate::public::structure::abstract_data::AbstractData;
    use crate::public::structure::album::share::Share;
    use crate::public::structure::object::next_mutation_timestamp;
    use crate::router::AppResult;
    use crate::storage::cache::{
        EXPIRE_CACHE_BYTES, QUERY_SNAPSHOT_CACHE_BYTES, TREE_SNAPSHOT_CACHE_BYTES,
    };
    use crate::tasks::batcher::update_tree::update_tree_task;
    use redb::ReadableDatabase;
    use rocket::Request;
    use rocket::Route;
    use rocket::http::Status;
    use rocket::request::{FromRequest, Outcome};
    use rocket::serde::json::Json;
    use rocket::serde::{Deserialize, Serialize};
    use std::collections::BTreeSet;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    const TOKEN_HEADER: &str = "x-urocissa-perf-token";
    const BARRIER_TIMEOUT: Duration = Duration::from_secs(30);
    const DRAIN_TIMEOUT: Duration = Duration::from_secs(300);

    pub struct PerfToken;

    #[rocket::async_trait]
    impl<'r> FromRequest<'r> for PerfToken {
        type Error = Status;

        async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
            let expected = std::env::var("UROCISSA_PERF_TOKEN").ok();
            let actual = request.headers().get_one(TOKEN_HEADER);
            if expected
                .as_deref()
                .is_some_and(|value| Some(value) == actual)
            {
                Outcome::Success(Self)
            } else {
                Outcome::Error((Status::Forbidden, Status::Forbidden))
            }
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "rocket::serde")]
    pub struct FixtureRequest {
        pub count: usize,
        pub seed: u64,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(crate = "rocket::serde")]
    pub struct FixtureSummary {
        pub requested: usize,
        pub inserted: usize,
        pub seed: u64,
        pub generation_ns: u64,
        pub insert_ns: u64,
        pub rebuild_ns: u64,
        pub total_ns: u64,
        pub expected_home: usize,
        pub expected_all: usize,
        pub expected_videos: usize,
        pub expected_favorites: usize,
        pub expected_archived: usize,
        pub expected_trashed: usize,
        pub database_bytes: u64,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(crate = "rocket::serde")]
    pub struct DeleteSummary {
        pub found: usize,
        pub deleted: usize,
        pub remaining: usize,
        pub scan_ns: u64,
        pub delete_ns: u64,
        pub rebuild_ns: u64,
        pub total_ns: u64,
        pub database_bytes: u64,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(crate = "rocket::serde")]
    pub struct StatusSummary {
        pub disk_count: usize,
        pub memory_count: usize,
        pub tree_snapshot_pending: usize,
        pub query_snapshot_pending: usize,
        pub database_bytes: u64,
        pub write_behind_pending_operations: usize,
        pub write_behind_flush_chunk_records: usize,
        pub write_behind_pending_records: usize,
        pub write_behind_active_records: usize,
        pub write_behind_flushing_records: usize,
        pub write_behind_pending_bytes: usize,
        pub write_behind_last_flush_records: usize,
        pub write_behind_last_flush_unique_records: usize,
        pub write_behind_last_flush_chunks: usize,
        pub write_behind_flush_records_per_second: Option<f64>,
        pub write_behind_estimated_drain_ms: Option<u64>,
        pub write_behind_flush_failure_count: u64,
        pub write_behind_flush_retry_count: u64,
        pub write_behind_last_error: Option<String>,
        pub backend_rss_bytes: u64,
        pub backend_global_peak_rss_bytes: u64,
        pub backend_phase_peak_rss_bytes: u64,
        pub backend_phase_average_rss_bytes: u64,
        pub backend_phase_rss_sample_count: u64,
        pub redb_main_cache: RedbCacheSummary,
        pub redb_tree_snapshot_cache: RedbCacheSummary,
        pub redb_query_snapshot_cache: RedbCacheSummary,
        pub redb_expire_cache: RedbCacheSummary,
        pub tree_memory: TreeMemorySummary,
        pub tree_snapshot_memory_bytes: usize,
        pub query_snapshot_memory_bytes: usize,
        pub write_behind_memory_bytes: usize,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(crate = "rocket::serde")]
    pub struct RedbCacheSummary {
        pub limit_bytes: usize,
        pub used_bytes: usize,
        pub evictions: u64,
        pub read_hits: u64,
        pub read_misses: u64,
        pub write_hits: u64,
        pub write_misses: u64,
    }

    #[derive(Debug, Clone, Copy, Serialize)]
    #[serde(crate = "rocket::serde")]
    pub struct TreeMemorySummary {
        pub arena_inline_bytes: usize,
        pub record_dynamic_bytes: usize,
        pub id_index_bytes: usize,
        pub order_index_bytes: usize,
        pub query_indexes_bytes: usize,
        pub album_catalog_bytes: usize,
        pub total_bytes: usize,
    }

    impl From<TreeMemoryUsage> for TreeMemorySummary {
        fn from(value: TreeMemoryUsage) -> Self {
            Self {
                arena_inline_bytes: value.arena_inline_bytes,
                record_dynamic_bytes: value.record_dynamic_bytes,
                id_index_bytes: value.id_index_bytes,
                order_index_bytes: value.order_index_bytes,
                query_indexes_bytes: value.query_indexes_bytes,
                album_catalog_bytes: value.album_catalog_bytes,
                total_bytes: value.total_bytes(),
            }
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct RestartProbeRequest {
        pub marker_tag: String,
        pub commits_before_failure: usize,
        #[serde(default)]
        pub target_limit: Option<usize>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct RestartProbeSummary {
        pub targets: usize,
        pub expected_durable_min: usize,
        pub expected_durable_max: usize,
        pub flush_chunk_records: usize,
        pub failure_count_before: u64,
        pub retry_count_before: u64,
    }

    #[derive(Debug, Clone, Copy, Deserialize, Default)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub enum AuditView {
        Logical,
        #[default]
        Disk,
    }

    #[derive(Debug, Clone, Deserialize, Default)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct AuditRequest {
        pub item_id: Option<String>,
        pub album_id: Option<String>,
        pub marker_tag: Option<String>,
        pub share_id: Option<String>,
        #[serde(default)]
        pub view: AuditView,
    }

    #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct AuditItemSummary {
        pub id: String,
        pub description: Option<String>,
        pub tags: Vec<String>,
        pub albums: Vec<String>,
        pub is_favorite: bool,
        pub is_archived: bool,
        pub is_trashed: bool,
    }

    #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct AuditAlbumSummary {
        pub id: String,
        pub title: Option<String>,
        pub cover: Option<String>,
        pub item_count: usize,
        pub scanned_member_count: usize,
        pub share_count: usize,
        pub share: Option<Share>,
    }

    #[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct AuditMarkerSummary {
        pub total: usize,
        pub favorite: usize,
        pub archived: usize,
        pub trashed: usize,
        pub album_members: usize,
    }

    #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct AuditSummary {
        pub disk_count: usize,
        pub item: Option<AuditItemSummary>,
        pub album: Option<AuditAlbumSummary>,
        pub marker: AuditMarkerSummary,
    }

    pub fn routes() -> Vec<Route> {
        routes![
            fixture,
            delete_fixture,
            status,
            phase,
            barrier,
            drain,
            restart_probe,
            audit
        ]
    }

    #[post("/__perf/fixture", format = "json", data = "<request>")]
    pub async fn fixture(
        _token: PerfToken,
        request: Json<FixtureRequest>,
    ) -> AppResult<Json<FixtureSummary>> {
        ensure_perf_root()?;
        if request.count == 0 || request.count > 2_000_000 {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "count must be between 1 and 2,000,000",
            ));
        }

        let request = request.into_inner();
        let initial = status_sync();
        if initial.disk_count != 0 || initial.memory_count != 0 {
            return Err(AppError::new(
                ErrorKind::Conflict,
                "performance fixture root is not empty",
            ));
        }
        let result = tokio::task::spawn_blocking(move || create_fixture(request))
            .await
            .map_err(|error| AppError::new(ErrorKind::Internal, error.to_string()))??;
        Ok(Json(result))
    }

    #[delete("/__perf/fixture")]
    pub async fn delete_fixture(_token: PerfToken) -> AppResult<Json<DeleteSummary>> {
        ensure_perf_root()?;
        if !WRITE_BEHIND.flush(DRAIN_TIMEOUT).await {
            return Err(AppError::new(
                ErrorKind::Internal,
                "write-behind did not drain before fixture cleanup",
            ));
        }
        let result = tokio::task::spawn_blocking(delete_fixture_sync)
            .await
            .map_err(|error| AppError::new(ErrorKind::Internal, error.to_string()))??;
        Ok(Json(result))
    }

    #[get("/__perf/status")]
    pub fn status(_token: PerfToken) -> AppResult<Json<StatusSummary>> {
        ensure_perf_root()?;
        Ok(Json(status_sync()))
    }

    #[post("/__perf/phase", format = "json", data = "<phase_request>")]
    pub fn phase(_token: PerfToken, phase_request: Json<PhaseRequest>) -> AppResult<Json<()>> {
        performance::set_phase(&phase_request.name);
        performance::flush();
        Ok(Json(()))
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(crate = "rocket::serde")]
    pub struct PhaseRequest {
        pub name: String,
    }

    #[post("/__perf/barrier")]
    pub async fn barrier(_token: PerfToken) -> AppResult<Json<StatusSummary>> {
        ensure_perf_root()?;
        let deadline = Instant::now() + BARRIER_TIMEOUT;
        loop {
            let current = status_sync();
            if current.tree_snapshot_pending == 0 && current.query_snapshot_pending == 0 {
                performance::flush();
                return Ok(Json(current));
            }
            if Instant::now() >= deadline {
                return Err(AppError::new(
                    ErrorKind::Internal,
                    "performance barrier timed out",
                ));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[post("/__perf/drain")]
    pub async fn drain(_token: PerfToken) -> AppResult<Json<StatusSummary>> {
        ensure_perf_root()?;
        if !WRITE_BEHIND.flush(DRAIN_TIMEOUT).await {
            return Err(AppError::new(
                ErrorKind::Internal,
                "performance write-behind drain timed out",
            ));
        }
        performance::flush();
        Ok(Json(status_sync()))
    }

    /// Publish a logical marker edit, then fail one flush transaction after
    /// the requested number of successful chunk commits. The benchmark kills
    /// this process after observing the failure and audits Redb after restart.
    #[post("/__perf/restart-probe", format = "json", data = "<request>")]
    pub async fn restart_probe(
        _token: PerfToken,
        request: Json<RestartProbeRequest>,
    ) -> AppResult<Json<RestartProbeSummary>> {
        ensure_perf_root()?;
        let request = request.into_inner();
        if request.marker_tag.is_empty() || request.marker_tag.len() > 128 {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "markerTag must contain between 1 and 128 bytes",
            ));
        }
        if WRITE_BEHIND.status().pending_operations != 0 {
            return Err(AppError::new(
                ErrorKind::Conflict,
                "write-behind must be drained before a restart probe",
            ));
        }

        let targets = {
            let state = TREE
                .state
                .read()
                .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
            let slot_refs = state
                .order
                .iter()
                .copied()
                .filter(|slot_ref| {
                    state.get(*slot_ref).is_some_and(|record| {
                        record.object_type != crate::public::structure::object::ObjectType::Album
                    })
                })
                .take(request.target_limit.unwrap_or(usize::MAX));
            TargetSet::from_slot_refs(slot_refs, state.arena.capacity())
        };
        if targets.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "restart probe target set is empty",
            ));
        }
        let operation = DirtyOperation::Tags {
            targets: targets.clone(),
            add: BTreeSet::from([request.marker_tag]),
            remove: BTreeSet::new(),
        };
        let touch = DirtyOperation::Touch {
            targets: targets.clone(),
            changed_at: next_mutation_timestamp(),
        };
        let bytes = operation.estimated_bytes() + touch.estimated_bytes();
        WRITE_BEHIND.reserve(bytes).await?;

        let mut state = match TREE.state.write() {
            Ok(state) => state,
            Err(_) => {
                WRITE_BEHIND.release_reservation(bytes);
                return Err(AppError::new(
                    ErrorKind::Internal,
                    "tree state lock poisoned",
                ));
            }
        };
        if !targets.is_current(&state) {
            WRITE_BEHIND.release_reservation(bytes);
            return Err(AppError::new(
                ErrorKind::Conflict,
                "restart probe targets became stale before publication",
            ));
        }
        let universe = state.arena.capacity();
        if let DirtyOperation::Tags {
            targets,
            add,
            remove,
            ..
        } = &operation
        {
            state
                .query
                .edit_tags(targets.ordinals(), add, remove, universe);
        }
        let status = WRITE_BEHIND.status();
        WRITE_BEHIND.enqueue_reserved(operation, bytes);
        WRITE_BEHIND.enqueue_reserved(touch, 0);
        WRITE_BEHIND.inject_flush_failure_after_commits(request.commits_before_failure);
        VERSION_COUNT_TIMESTAMP.fetch_add(1, Ordering::Relaxed);
        drop(state);
        WRITE_BEHIND.wake();

        let committed = request
            .commits_before_failure
            .saturating_mul(FLUSH_CHUNK_SIZE)
            .min(targets.len());
        Ok(Json(RestartProbeSummary {
            targets: targets.len(),
            expected_durable_min: committed,
            expected_durable_max: committed,
            flush_chunk_records: FLUSH_CHUNK_SIZE,
            failure_count_before: status.flush_failure_count,
            retry_count_before: status.flush_retry_count,
        }))
    }

    #[post("/__perf/audit", format = "json", data = "<request>")]
    pub async fn audit(
        _token: PerfToken,
        request: Json<AuditRequest>,
    ) -> AppResult<Json<AuditSummary>> {
        ensure_perf_root()?;
        let request = request.into_inner();
        let result = tokio::task::spawn_blocking(move || audit_sync(&request))
            .await
            .map_err(|error| AppError::new(ErrorKind::Internal, error.to_string()))??;
        Ok(Json(result))
    }

    fn create_fixture(request: FixtureRequest) -> Result<FixtureSummary, AppError> {
        let total_start = Instant::now();
        let mut generation_ns = 0_u64;
        let mut insert_ns = 0_u64;
        let mut expected_home = 0_usize;
        let mut expected_all = 0_usize;
        let mut expected_videos = 0_usize;
        let mut expected_favorites = 0_usize;
        let mut expected_archived = 0_usize;
        let mut expected_trashed = 0_usize;
        let mut inserted = 0_usize;
        let mut batch = Vec::with_capacity(4_096);

        for index in 0..request.count as u64 {
            let generation_start = Instant::now();
            let item = AbstractData::generate_performance_data(index, request.seed);
            generation_ns = generation_ns.saturating_add(elapsed_ns(generation_start));
            expected_home += usize::from(!item.is_archived() && !item.is_trashed());
            expected_all += usize::from(!item.is_trashed());
            expected_videos +=
                usize::from(item.is_video() && !item.is_archived() && !item.is_trashed());
            expected_favorites += usize::from(item.is_favorite() && !item.is_trashed());
            expected_archived += usize::from(item.is_archived() && !item.is_trashed());
            expected_trashed += usize::from(item.is_trashed());
            batch.push(item);
            if batch.len() == 4_096 || index + 1 == request.count as u64 {
                let insert_start = Instant::now();
                TREE.store.write(|writer| {
                    for item in &batch {
                        writer.insert(item).map_err(|error| {
                            AppError::new(ErrorKind::Database, error.to_string())
                        })?;
                    }
                    Ok::<(), AppError>(())
                })?;
                insert_ns = insert_ns.saturating_add(elapsed_ns(insert_start));
                inserted += batch.len();
                batch.clear();
            }
        }

        let rebuild_start = Instant::now();
        update_tree_task()
            .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
        let rebuild_ns = elapsed_ns(rebuild_start);
        let status = status_sync();
        if status.disk_count != request.count || status.memory_count != request.count {
            return Err(AppError::new(
                ErrorKind::Database,
                format!(
                    "fixture count mismatch: expected {}, disk {}, memory {}",
                    request.count, status.disk_count, status.memory_count
                ),
            ));
        }

        Ok(FixtureSummary {
            requested: request.count,
            inserted,
            seed: request.seed,
            generation_ns,
            insert_ns,
            rebuild_ns,
            total_ns: elapsed_ns(total_start),
            expected_home,
            expected_all,
            expected_videos,
            expected_favorites,
            expected_archived,
            expected_trashed,
            database_bytes: status.database_bytes,
        })
    }

    fn delete_fixture_sync() -> Result<DeleteSummary, AppError> {
        let total_start = Instant::now();
        let mut scan_ns = 0_u64;
        let mut delete_ns = 0_u64;
        let mut found = 0_usize;
        loop {
            let scan_start = Instant::now();
            let keys = TREE.store.read(|reader| {
                reader
                    .iter()?
                    .take(4_096)
                    .map(|entry| {
                        let (key, _) = entry?;
                        Ok::<_, anyhow::Error>(key.value().to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()
            })?;
            scan_ns = scan_ns.saturating_add(elapsed_ns(scan_start));
            if keys.is_empty() {
                break;
            }
            found += keys.len();
            let delete_start = Instant::now();
            TREE.store.write(|writer| {
                for key in &keys {
                    writer
                        .remove(key)
                        .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
                }
                Ok::<(), AppError>(())
            })?;
            delete_ns = delete_ns.saturating_add(elapsed_ns(delete_start));
        }

        let rebuild_start = Instant::now();
        update_tree_task()
            .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
        let rebuild_ns = elapsed_ns(rebuild_start);
        let status = status_sync();

        Ok(DeleteSummary {
            found,
            deleted: found.saturating_sub(status.disk_count),
            remaining: status.disk_count,
            scan_ns,
            delete_ns,
            rebuild_ns,
            total_ns: elapsed_ns(total_start),
            database_bytes: status.database_bytes,
        })
    }

    fn status_sync() -> StatusSummary {
        let disk_count = open_data_table()
            .len()
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let (memory_count, tree_memory) = TREE
            .state
            .read()
            .map_or((0, TreeMemoryUsage::default()), |value| {
                (value.len(), value.memory_usage())
            });
        let database_bytes = std::fs::metadata(get_data_path().join("db/index_v6.redb"))
            .map_or(0, |metadata| metadata.len());
        let write_behind = WRITE_BEHIND.status();
        let memory = performance::memory_snapshot();
        let tree_snapshot_memory_bytes = std::mem::size_of_val(&*TREE_SNAPSHOT.in_memory)
            .saturating_add(
                TREE_SNAPSHOT
                    .in_memory
                    .iter()
                    .map(|entry| entry.value().estimated_bytes())
                    .sum::<usize>(),
            )
            .saturating_add(std::mem::size_of_val(&*TREE_SNAPSHOT.verified_layouts))
            .saturating_add(TREE_SNAPSHOT.verified_layouts.len().saturating_mul(
                std::mem::size_of::<i64>()
                    + std::mem::size_of::<crate::public::db::tree_snapshot::SnapshotBlobLayout>(),
            ));
        let query_snapshot_memory_bytes = std::mem::size_of_val(&*QUERY_SNAPSHOT.in_memory)
            .saturating_add(QUERY_SNAPSHOT.in_memory.len().saturating_mul(
                std::mem::size_of::<u64>()
                    + std::mem::size_of::<crate::router::get::get_prefetch::Prefetch>(),
            ));
        StatusSummary {
            disk_count,
            memory_count,
            tree_snapshot_pending: TREE_SNAPSHOT.in_memory.len(),
            query_snapshot_pending: QUERY_SNAPSHOT.in_memory.len(),
            database_bytes,
            write_behind_pending_operations: write_behind.pending_operations,
            write_behind_flush_chunk_records: write_behind.flush_chunk_records,
            write_behind_pending_records: write_behind.pending_records,
            write_behind_active_records: write_behind.active_records,
            write_behind_flushing_records: write_behind.flushing_records,
            write_behind_pending_bytes: write_behind.pending_bytes,
            write_behind_last_flush_records: write_behind.last_flush_records,
            write_behind_last_flush_unique_records: write_behind.last_flush_unique_records,
            write_behind_last_flush_chunks: write_behind.last_flush_chunks,
            write_behind_flush_records_per_second: write_behind.flush_records_per_second,
            write_behind_estimated_drain_ms: write_behind.estimated_drain_ms,
            write_behind_flush_failure_count: write_behind.flush_failure_count,
            write_behind_flush_retry_count: write_behind.flush_retry_count,
            write_behind_last_error: write_behind.last_error,
            backend_rss_bytes: memory.current_rss_bytes,
            backend_global_peak_rss_bytes: memory.global_peak_rss_bytes,
            backend_phase_peak_rss_bytes: memory.phase_peak_rss_bytes,
            backend_phase_average_rss_bytes: memory.phase_average_rss_bytes,
            backend_phase_rss_sample_count: memory.phase_sample_count,
            redb_main_cache: cache_summary(
                TREE.store.cache_limit_bytes(),
                TREE.store.cache_stats(),
            ),
            redb_tree_snapshot_cache: cache_summary(
                TREE_SNAPSHOT_CACHE_BYTES,
                TREE_SNAPSHOT.in_disk.cache_stats(),
            ),
            redb_query_snapshot_cache: cache_summary(
                QUERY_SNAPSHOT_CACHE_BYTES,
                QUERY_SNAPSHOT.in_disk.cache_stats(),
            ),
            redb_expire_cache: cache_summary(EXPIRE_CACHE_BYTES, EXPIRE.in_disk.cache_stats()),
            tree_memory: tree_memory.into(),
            tree_snapshot_memory_bytes,
            query_snapshot_memory_bytes,
            write_behind_memory_bytes: write_behind.pending_bytes,
        }
    }

    fn cache_summary(limit_bytes: usize, stats: redb::CacheStats) -> RedbCacheSummary {
        RedbCacheSummary {
            limit_bytes,
            used_bytes: stats.used_bytes(),
            evictions: stats.evictions(),
            read_hits: stats.read_hits(),
            read_misses: stats.read_misses(),
            write_hits: stats.write_hits(),
            write_misses: stats.write_misses(),
        }
    }

    fn audit_sync(request: &AuditRequest) -> Result<AuditSummary, AppError> {
        if matches!(request.view, AuditView::Logical) {
            return audit_logical(request);
        }
        let table = open_data_table();
        let records = table
            .iter()
            .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?
            .map(|entry| {
                let (_, value) =
                    entry.map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
                Ok::<_, AppError>(value.into_value())
            });
        build_audit(records, request)
    }

    fn audit_logical(request: &AuditRequest) -> Result<AuditSummary, AppError> {
        let state = TREE
            .state
            .read()
            .map_err(|_| AppError::new(ErrorKind::Internal, "tree state lock poisoned"))?;
        let marker_members = request
            .marker_tag
            .as_deref()
            .and_then(|tag| state.query.tags.get(tag));
        let marker = marker_members.map_or_else(AuditMarkerSummary::default, |members| {
            let album_members = request
                .album_id
                .as_deref()
                .and_then(|id| arrayvec::ArrayString::<64>::from(id).ok())
                .and_then(|id| state.query.albums.get(&id));
            AuditMarkerSummary {
                total: members.len(),
                favorite: members
                    .iter()
                    .filter(|ordinal| state.query.favorite.contains(*ordinal))
                    .count(),
                archived: members
                    .iter()
                    .filter(|ordinal| state.query.archived.contains(*ordinal))
                    .count(),
                trashed: members
                    .iter()
                    .filter(|ordinal| state.query.trashed.contains(*ordinal))
                    .count(),
                album_members: members
                    .iter()
                    .filter(|ordinal| album_members.is_some_and(|album| album.contains(*ordinal)))
                    .count(),
            }
        });
        let album = request.album_id.as_deref().and_then(|id| {
            let id = arrayvec::ArrayString::<64>::from(id).ok()?;
            let album = state.albums.get(&id)?;
            let share = request.share_id.as_deref().and_then(|share_id| {
                album
                    .metadata
                    .share_list
                    .iter()
                    .find(|(id, _)| id.as_str() == share_id)
                    .map(|(_, share)| share.clone())
            });
            Some(AuditAlbumSummary {
                id: id.to_string(),
                title: album.metadata.title.clone(),
                cover: album.metadata.cover.map(|cover| cover.to_string()),
                item_count: album.metadata.item_count,
                scanned_member_count: state.query.albums.get(&id).map_or(0, |set| set.len()),
                share_count: album.metadata.share_list.len(),
                share,
            })
        });
        let disk_count = open_data_table()
            .len()
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let item_id = request.item_id.clone();
        drop(state);
        let item = item_id
            .as_deref()
            .map(|id| {
                TREE.store.read(|reader| {
                    let durable = reader.get(id)?.map(|value| value.into_value());
                    Ok::<_, anyhow::Error>(WRITE_BEHIND.logical_record(id, durable))
                })
            })
            .transpose()
            .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?
            .flatten()
            .as_ref()
            .map(audit_item);
        Ok(AuditSummary {
            disk_count,
            item,
            album,
            marker,
        })
    }

    fn build_audit(
        records: impl IntoIterator<Item = Result<AbstractData, AppError>>,
        request: &AuditRequest,
    ) -> Result<AuditSummary, AppError> {
        let album_id = request.album_id.as_deref();
        let marker_tag = request.marker_tag.as_deref();
        let mut marker = AuditMarkerSummary::default();
        let mut disk_count = 0_usize;
        let mut item = None;
        let mut album = None;
        let mut scanned_member_count = 0_usize;

        for record in records {
            let record = record?;
            disk_count += 1;
            if marker_tag.is_some_and(|tag| record.tag().contains(tag)) {
                marker.total += 1;
                marker.favorite += usize::from(record.is_favorite());
                marker.archived += usize::from(record.is_archived());
                marker.trashed += usize::from(record.is_trashed());
                marker.album_members += usize::from(album_id.is_some_and(|id| {
                    record
                        .albums()
                        .is_some_and(|albums| albums.iter().any(|album| album.as_str() == id))
                }));
            }
            if album_id.is_some_and(|id| {
                record
                    .albums()
                    .is_some_and(|albums| albums.iter().any(|candidate| candidate.as_str() == id))
            }) {
                scanned_member_count += 1;
            }
            if item.is_none()
                && request
                    .item_id
                    .as_deref()
                    .is_some_and(|id| record.hash().as_str() == id)
            {
                item = Some(audit_item(&record));
            }
            if album.is_none() {
                if let AbstractData::Album(album_record) = &record {
                    if !album_id.is_some_and(|id| album_record.object.id.as_str() == id) {
                        continue;
                    }
                    let share = request.share_id.as_deref().and_then(|share_id| {
                        album_record
                            .metadata
                            .share_list
                            .iter()
                            .find(|(id, _)| id.as_str() == share_id)
                            .map(|(_, share)| share.clone())
                    });
                    album = Some(AuditAlbumSummary {
                        id: album_record.object.id.to_string(),
                        title: album_record.metadata.title.clone(),
                        cover: album_record.metadata.cover.map(|cover| cover.to_string()),
                        item_count: album_record.metadata.item_count,
                        scanned_member_count: 0,
                        share_count: album_record.metadata.share_list.len(),
                        share,
                    });
                }
            }
        }
        if let Some(album) = &mut album {
            album.scanned_member_count = scanned_member_count;
        }

        Ok(AuditSummary {
            disk_count,
            item,
            album,
            marker,
        })
    }

    fn audit_item(record: &AbstractData) -> AuditItemSummary {
        let description = match record {
            AbstractData::Image(data) => data.object.description.clone(),
            AbstractData::Video(data) => data.object.description.clone(),
            AbstractData::Album(data) => data.object.description.clone(),
        };
        let mut tags = record.tag().iter().cloned().collect::<Vec<_>>();
        tags.sort_unstable();
        let mut albums = record.albums().map_or_else(Vec::new, |albums| {
            albums.iter().map(ToString::to_string).collect::<Vec<_>>()
        });
        albums.sort_unstable();
        AuditItemSummary {
            id: record.hash().to_string(),
            description,
            tags,
            albums,
            is_favorite: record.is_favorite(),
            is_archived: record.is_archived(),
            is_trashed: record.is_trashed(),
        }
    }

    fn ensure_perf_root() -> Result<(), AppError> {
        let root = get_data_path();
        if root.join(".urocissa-performance-root").is_file() {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorKind::PermissionDenied,
                "performance root marker is missing",
            ))
        }
    }

    fn elapsed_ns(start: Instant) -> u64 {
        start.elapsed().as_nanos().min(u64::MAX as u128) as u64
    }

    #[cfg(test)]
    mod tests {
        use super::{AuditRequest, build_audit};
        use crate::public::structure::abstract_data::AbstractData;
        use crate::public::structure::album::{album::Album, share::Share};
        use arrayvec::ArrayString;

        #[test]
        fn audit_reports_marker_item_album_and_share_state() {
            let album_id = ArrayString::<64>::from("performance-album").unwrap();
            let share_id = ArrayString::<64>::from("performance-share").unwrap();
            let mut first = AbstractData::generate_performance_data(1, 7);
            let mut second = AbstractData::generate_performance_data(2, 7);
            first.tag_mut().insert("benchmark-marker".to_string());
            second.tag_mut().insert("benchmark-marker".to_string());
            first.albums_mut().unwrap().insert(album_id);
            if let AbstractData::Image(data) = &mut first {
                data.object.description = Some("benchmark-description".to_string());
                data.object.is_favorite = true;
                data.object.is_archived = false;
                data.object.is_trashed = false;
            } else if let AbstractData::Video(data) = &mut first {
                data.object.description = Some("benchmark-description".to_string());
                data.object.is_favorite = true;
                data.object.is_archived = false;
                data.object.is_trashed = false;
            }
            if let AbstractData::Image(data) = &mut second {
                data.object.is_favorite = false;
                data.object.is_archived = true;
                data.object.is_trashed = true;
            } else if let AbstractData::Video(data) = &mut second {
                data.object.is_favorite = false;
                data.object.is_archived = true;
                data.object.is_trashed = true;
            }

            let item_id = first.hash().to_string();
            let mut album =
                Album::new(album_id, Some("Benchmark Album".to_string())).into_abstract_data();
            if let AbstractData::Album(data) = &mut album {
                data.metadata.item_count = 1;
                data.metadata.cover = Some(first.hash());
                data.metadata.share_list.insert(
                    share_id,
                    Share {
                        url: share_id,
                        description: "updated share".to_string(),
                        show_download: true,
                        ..Share::default()
                    },
                );
            }

            let summary = build_audit(
                [first, second, album].into_iter().map(Ok),
                &AuditRequest {
                    item_id: Some(item_id.clone()),
                    album_id: Some(album_id.to_string()),
                    marker_tag: Some("benchmark-marker".to_string()),
                    share_id: Some(share_id.to_string()),
                    ..AuditRequest::default()
                },
            )
            .unwrap();

            assert_eq!(summary.disk_count, 3);
            assert_eq!(summary.marker.total, 2);
            assert_eq!(summary.marker.favorite, 1);
            assert_eq!(summary.marker.archived, 1);
            assert_eq!(summary.marker.trashed, 1);
            assert_eq!(summary.marker.album_members, 1);
            assert_eq!(summary.item.unwrap().id, item_id);
            let album = summary.album.unwrap();
            assert_eq!(album.title.as_deref(), Some("Benchmark Album"));
            assert_eq!(album.scanned_member_count, 1);
            assert_eq!(album.share_count, 1);
            assert_eq!(album.share.unwrap().description, "updated share");
        }

        #[test]
        fn audit_returns_empty_optional_records_for_unknown_ids() {
            let records = vec![AbstractData::generate_performance_data(1, 7)];
            let summary = build_audit(
                records.into_iter().map(Ok),
                &AuditRequest {
                    item_id: Some("missing-item".to_string()),
                    album_id: Some("missing-album".to_string()),
                    marker_tag: Some("missing-tag".to_string()),
                    share_id: Some("missing-share".to_string()),
                    ..AuditRequest::default()
                },
            )
            .unwrap();
            assert!(summary.item.is_none());
            assert!(summary.album.is_none());
            assert_eq!(summary.marker.total, 0);
        }
    }
}

#[cfg(feature = "performance-test")]
pub fn generate_performance_routes() -> Vec<Route> {
    enabled::routes()
}

#[cfg(not(feature = "performance-test"))]
pub fn generate_performance_routes() -> Vec<Route> {
    Vec::new()
}
