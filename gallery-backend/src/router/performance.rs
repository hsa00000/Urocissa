use rocket::Route;

#[cfg(feature = "performance-test")]
mod enabled {
    use crate::operations::open_db::open_data_table;
    use crate::performance;
    use crate::public::constant::storage::get_data_path;
    use crate::public::db::{
        query_snapshot::QUERY_SNAPSHOT, tree::TREE, tree_snapshot::TREE_SNAPSHOT,
    };
    use crate::public::error::{AppError, ErrorKind};
    use crate::public::structure::abstract_data::AbstractData;
    use crate::public::structure::album::share::Share;
    use crate::router::AppResult;
    use crate::tasks::batcher::update_tree::update_tree_task;
    use rocket::Request;
    use rocket::Route;
    use rocket::http::Status;
    use rocket::request::{FromRequest, Outcome};
    use rocket::serde::json::Json;
    use rocket::serde::{Deserialize, Serialize};
    use std::time::{Duration, Instant};

    const TOKEN_HEADER: &str = "x-urocissa-perf-token";
    const BARRIER_TIMEOUT: Duration = Duration::from_secs(30);

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
    }

    #[derive(Debug, Clone, Deserialize, Default)]
    #[serde(crate = "rocket::serde", rename_all = "camelCase")]
    pub struct AuditRequest {
        pub item_id: Option<String>,
        pub album_id: Option<String>,
        pub marker_tag: Option<String>,
        pub share_id: Option<String>,
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
        routes![fixture, delete_fixture, status, phase, barrier, audit]
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
        let generation_start = Instant::now();
        let data = (0..request.count as u64)
            .map(|index| AbstractData::generate_performance_data(index, request.seed))
            .collect::<Vec<_>>();
        let generation_ns = elapsed_ns(generation_start);

        let expected_home = data
            .iter()
            .filter(|item| !item.is_archived() && !item.is_trashed())
            .count();
        let expected_all = data.iter().filter(|item| !item.is_trashed()).count();
        let expected_videos = data
            .iter()
            .filter(|item| item.is_video() && !item.is_archived() && !item.is_trashed())
            .count();
        let expected_favorites = data
            .iter()
            .filter(|item| item.is_favorite() && !item.is_trashed())
            .count();
        let expected_archived = data
            .iter()
            .filter(|item| item.is_archived() && !item.is_trashed())
            .count();
        let expected_trashed = data.iter().filter(|item| item.is_trashed()).count();

        let insert_start = Instant::now();
        TREE.store.write(|writer| {
            for item in &data {
                writer
                    .insert(item)
                    .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
            }
            Ok::<(), AppError>(())
        })?;
        let insert_ns = elapsed_ns(insert_start);

        let rebuild_start = Instant::now();
        update_tree_task();
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
            inserted: data.len(),
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
        let scan_start = Instant::now();
        let data = open_data_table()
            .iter()
            .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?
            .map(|entry| {
                let (_, value) =
                    entry.map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
                Ok::<_, AppError>(value.value())
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let scan_ns = elapsed_ns(scan_start);

        let delete_start = Instant::now();
        TREE.store.write(|writer| {
            for item in &data {
                let hash = item.hash();
                writer
                    .remove(hash.as_str())
                    .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
            }
            Ok::<(), AppError>(())
        })?;
        let delete_ns = elapsed_ns(delete_start);

        let rebuild_start = Instant::now();
        update_tree_task();
        let rebuild_ns = elapsed_ns(rebuild_start);
        let status = status_sync();

        Ok(DeleteSummary {
            found: data.len(),
            deleted: data.len().saturating_sub(status.disk_count),
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
        let memory_count = TREE.in_memory.read().map_or(0, |value| value.len());
        let database_bytes = std::fs::metadata(get_data_path().join("db/index_v6.redb"))
            .map_or(0, |metadata| metadata.len());
        StatusSummary {
            disk_count,
            memory_count,
            tree_snapshot_pending: TREE_SNAPSHOT.in_memory.len(),
            query_snapshot_pending: QUERY_SNAPSHOT.in_memory.len(),
            database_bytes,
        }
    }

    fn audit_sync(request: &AuditRequest) -> Result<AuditSummary, AppError> {
        let records = open_data_table()
            .iter()
            .map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?
            .map(|entry| {
                let (_, value) =
                    entry.map_err(|error| AppError::new(ErrorKind::Database, error.to_string()))?;
                Ok::<_, AppError>(value.value())
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(build_audit(&records, request))
    }

    fn build_audit(records: &[AbstractData], request: &AuditRequest) -> AuditSummary {
        let album_id = request.album_id.as_deref();
        let marker_tag = request.marker_tag.as_deref();
        let mut marker = AuditMarkerSummary::default();

        for record in records {
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
        }

        let item = request.item_id.as_deref().and_then(|id| {
            records
                .iter()
                .find(|record| record.hash().as_str() == id)
                .map(audit_item)
        });
        let album = album_id.and_then(|id| {
            records.iter().find_map(|record| match record {
                AbstractData::Album(album) if album.object.id.as_str() == id => {
                    let scanned_member_count = records
                        .iter()
                        .filter(|record| {
                            record.albums().is_some_and(|albums| {
                                albums.iter().any(|album_id| album_id.as_str() == id)
                            })
                        })
                        .count();
                    let share = request.share_id.as_deref().and_then(|share_id| {
                        album
                            .metadata
                            .share_list
                            .iter()
                            .find(|(id, _)| id.as_str() == share_id)
                            .map(|(_, share)| share.clone())
                    });
                    Some(AuditAlbumSummary {
                        id: album.object.id.to_string(),
                        title: album.metadata.title.clone(),
                        cover: album.metadata.cover.map(|cover| cover.to_string()),
                        item_count: album.metadata.item_count,
                        scanned_member_count,
                        share_count: album.metadata.share_list.len(),
                        share,
                    })
                }
                _ => None,
            })
        });

        AuditSummary {
            disk_count: records.len(),
            item,
            album,
            marker,
        }
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
                &[first, second, album],
                &AuditRequest {
                    item_id: Some(item_id.clone()),
                    album_id: Some(album_id.to_string()),
                    marker_tag: Some("benchmark-marker".to_string()),
                    share_id: Some(share_id.to_string()),
                },
            );

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
                &records,
                &AuditRequest {
                    item_id: Some("missing-item".to_string()),
                    album_id: Some("missing-album".to_string()),
                    marker_tag: Some("missing-tag".to_string()),
                    share_id: Some("missing-share".to_string()),
                },
            );
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
