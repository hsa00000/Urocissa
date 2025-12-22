use anyhow::{Context, Result, anyhow, bail};
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use log::{error, info, warn};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rocket::form::{Errors, Form, FromForm};
use rocket::fs::{NamedFile, TempFile};
use rocket::http::Status;
use rocket::response::{Redirect, Responder, content};
use rocket::serde::json::Json;
use rocket::{get, post, put};
use rocket_seek_stream::SeekStream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::Ordering;

use redb::ReadableDatabase;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::task::spawn_blocking;
use uuid::Uuid;

use crate::api::claims::timestamp::ClaimsTimestamp;
use crate::api::fairings::guards::auth::GuardAuth;
use crate::api::fairings::guards::hash::{GuardHash, GuardHashOriginal};
use crate::api::fairings::guards::readonly::GuardReadOnlyMode;
use crate::api::fairings::guards::share::GuardShare;
use crate::api::fairings::guards::timestamp::GuardTimestamp;
use crate::api::fairings::guards::upload::GuardUpload;
use crate::api::{AppResult, GuardResult};
use crate::background::actors::{BATCH_COORDINATOR, INDEX_COORDINATOR};
use crate::background::batchers::flush_query::FlushQuerySnapshotTask;
use crate::background::batchers::flush_snapshot::FlushTreeSnapshotTask;
use crate::background::batchers::flush_tree::FlushTreeTask;
use crate::background::batchers::update_tree::UpdateTreeTask;
use crate::background::flows::index_workflow;
use crate::background::processors::image::{
    generate_dynamic_image, generate_phash, generate_thumbhash,
};
use crate::background::processors::transitor::{
    index_to_hash, process_abstract_data_for_response, resolve_show_download_and_metadata,
};
use crate::common::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};
use crate::config::{PUBLIC_CONFIG, PublicConfig};
use crate::database::ops::snapshot::query::QUERY_SNAPSHOT;
use crate::database::ops::snapshot::tree::TREE_SNAPSHOT;
use crate::database::ops::tree::tags::TagInfo;
use crate::database::ops::tree::{TREE, VERSION_COUNT_TIMESTAMP};
use crate::database::schema::relations::album_share::{AlbumShareTable, ResolvedShare, Share};
use crate::models::dto::reduced_data::ReducedData;
use crate::models::entity::abstract_data::{AbstractData, AbstractDataResponse};
use crate::models::entity::row::{Row, ScrollBarData};
use crate::models::filter::Expression;
use crate::utils::{PathExt, imported_path, thumbnail_path};

#[derive(Responder)]
pub enum CompressedFileResponse<'a> {
    SeekStream(SeekStream<'a>),
    NamedFile(NamedFile),
}

#[get("/object/compressed/<file_path..>")]
pub async fn compressed_file(
    auth_guard: GuardResult<GuardShare>,
    hash_guard: GuardResult<GuardHash>,
    file_path: PathBuf,
) -> AppResult<CompressedFileResponse<'static>> {
    let _ = auth_guard?;
    let _ = hash_guard?;
    let compressed_file_path = Path::new("./object/compressed").join(&file_path);

    let ext = compressed_file_path.ext_lower();
    let result = match ext.as_str() {
        "mp4" => SeekStream::from_path(&compressed_file_path)
            .map(CompressedFileResponse::SeekStream)
            .context(format!(
                "Failed to open MP4 file: {}",
                compressed_file_path.display()
            ))?,
        "jpg" => {
            let named_file = NamedFile::open(&compressed_file_path)
                .await
                .context(format!(
                    "Failed to open JPG file: {}",
                    compressed_file_path.display()
                ))?;
            CompressedFileResponse::NamedFile(named_file)
        }
        "" => {
            return Err(anyhow::anyhow!("File has no extension")
                .context(format!("File path: {}", compressed_file_path.display()))
                .into());
        }
        ext => {
            return Err(anyhow::anyhow!("Unsupported file extension: {}", ext)
                .context(format!("File path: {}", compressed_file_path.display()))
                .into());
        }
    };

    Ok(result)
}

#[get("/object/imported/<file_path..>")]
pub async fn imported_file(
    auth: GuardResult<GuardShare>,
    hash_guard: GuardResult<GuardHashOriginal>,
    file_path: PathBuf,
) -> AppResult<CompressedFileResponse<'static>> {
    let _ = auth?;
    let _ = hash_guard?;
    let imported_file_path = Path::new("./object/imported").join(&file_path);
    NamedFile::open(imported_file_path)
        .await
        .map(CompressedFileResponse::NamedFile)
        .map_err(|error| {
            error!("Error opening imported file: {:#?}", error);
            anyhow::anyhow!("Error opening imported file: {:#?}", error).into()
        })
}

#[get("/get/get-config.json")]
pub async fn get_config(auth: GuardResult<GuardShare>) -> AppResult<Json<&'static PublicConfig>> {
    let _ = auth?;
    Ok(Json(&*PUBLIC_CONFIG))
}

#[get("/get/get-tags")]
pub async fn get_tags(auth: GuardResult<GuardAuth>) -> AppResult<Json<Vec<TagInfo>>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || {
        let vec_tags_info = TREE.read_tags();
        Ok(Json(vec_tags_info))
    })
    .await?
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumInfo {
    pub album_id: String,
    pub album_name: Option<String>,
    pub share_list: HashMap<ArrayString<64>, Share>,
}

#[get("/get/get-albums")]
pub async fn get_albums(auth: GuardResult<GuardAuth>) -> AppResult<Json<Vec<AlbumInfo>>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || {
        let txn = TREE.begin_read()?;
        let album_list = TREE.read_albums().context("Failed to read albums")?;
        let mut all_shares_map =
            AlbumShareTable::get_all_shares_grouped(&txn).context("Failed to fetch shares")?;
        let album_info_list = album_list
            .into_iter()
            .map(|album| {
                let share_list = all_shares_map
                    .remove(album.object.id.as_str())
                    .unwrap_or_default();
                AlbumInfo {
                    album_id: album.object.id.to_string(),
                    album_name: album.metadata.title,
                    share_list,
                }
            })
            .collect();
        Ok(Json(album_info_list))
    })
    .await?
}

#[get("/get/get-all-shares")]
pub async fn get_all_shares(auth: GuardResult<GuardAuth>) -> AppResult<Json<Vec<ResolvedShare>>> {
    let _ = auth?;
    tokio::task::spawn_blocking(move || {
        let txn = TREE.begin_read()?;
        let shares =
            AlbumShareTable::get_all_resolved(&txn).context("Failed to read all shares")?;
        Ok(Json(shares))
    })
    .await?
}

use std::time::Duration; // 確保引入 Duration

#[get("/get/get-data?<timestamp>&<start>&<end>")]
pub async fn get_data(
    guard_timestamp: GuardResult<GuardTimestamp>,
    timestamp: u128,
    start: usize,
    mut end: usize,
) -> AppResult<Json<Vec<AbstractDataResponse>>> {
    let guard_timestamp = guard_timestamp?;

    tokio::task::spawn_blocking(move || {
        let total_start_time = Instant::now();

        let resolved_share_opt = guard_timestamp.claims.resolved_share_opt;
        let (show_download, show_metadata) = resolve_show_download_and_metadata(resolved_share_opt);

        // 1. 開啟 Transaction (由最外層持有)
        let t_txn = Instant::now();
        let txn = TREE.begin_read()?; // 這是一個 ReadTransaction
        let tree_snapshot_txn = TREE_SNAPSHOT.in_disk.begin_read()?; // 這是 Snapshot DB 的 txn
        let txn_open_cost = t_txn.elapsed();

        // Step A: Snapshot Read
        let t_snapshot = Instant::now();
        // 傳入 snapshot txn
        let tree_snapshot = TREE_SNAPSHOT.read_tree_snapshot(&tree_snapshot_txn, &timestamp)?;
        
        // 注意：len() 現在回傳 Result
        let total_len = tree_snapshot.len()?; 
        end = end.min(total_len);
        let snapshot_cost = t_snapshot.elapsed();

        let mut response_list = Vec::with_capacity(end - start);

        // 定義累加計時器
        let mut hash_lookup_cost = Duration::new(0, 0);
        let mut db_load_cost = Duration::new(0, 0);
        let mut process_data_cost = Duration::new(0, 0);
        let mut alias_lookup_cost = Duration::new(0, 0); // 對應 to_response

        for index in start..end {
            // 2. 索引轉 Hash (Snapshot 查詢)
            let t1 = Instant::now();
            let hash = index_to_hash(&tree_snapshot, index)?;
            hash_lookup_cost += t1.elapsed();

            // 3. 從 DB 讀取主資料 (Load Object/Metadata & Deserialize)
            let t2 = Instant::now();
            let abstract_data = TREE.load_from_txn(&txn, &hash)?;
            db_load_cost += t2.elapsed();

            // 4. 清除敏感資料邏輯 (純記憶體操作)
            let t3 = Instant::now();
            let processed_data = process_abstract_data_for_response(abstract_data, show_metadata);
            process_data_cost += t3.elapsed();

            // 5. 轉換回應 (包含 Alias 查詢)
            let t4 = Instant::now();
            response_list.push(processed_data.to_response(
                &txn,
                guard_timestamp.claims.timestamp,
                show_download,
            ));
            alias_lookup_cost += t4.elapsed();
        }
        // --- 優化與 Debug 結束 ---

        let total_duration = total_start_time.elapsed();

        // 印出詳細的時間分析
        info!(
            "Get Data Performance ({}-{}): \n\
            - Total Time: {:?}\n\
            - Snapshot Read: {:?}\n\
            - Txn Open: {:?}\n\
            - Index->Hash: {:?}\n\
            - DB Load (Data): {:?}\n\
            - Logic Process: {:?}\n\
            - DB Load (Alias/Token): {:?}",
            start,
            end,
            total_duration,
            snapshot_cost,
            txn_open_cost,
            hash_lookup_cost,
            db_load_cost,
            process_data_cost,
            alias_lookup_cost
        );

        Ok(Json(response_list))
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

pub static INDEX_HTML: LazyLock<String> = LazyLock::new(|| {
    fs::read_to_string("../gallery-frontend/dist/index.html").expect("Unable to read index.html")
});

#[get("/")]
pub fn redirect_to_photo() -> content::RawHtml<String> {
    content::RawHtml(INDEX_HTML.to_string())
}

#[get("/login")]
pub async fn login() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/redirect-to-login")]
pub fn redirect_to_login() -> Redirect {
    Redirect::to(uri!("/login"))
}

#[get("/unauthorized")]
pub async fn unauthorized() -> Status {
    Status::Unauthorized
}

#[get("/home")]
pub async fn home() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/home/view/<_path..>")]
pub async fn home_view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/favorite")]
pub async fn favorite() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/favorite/view/<_path..>")]
pub async fn favorite_view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/albums")]
pub async fn albums() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/albums/view/<_path..>")]
pub async fn albums_view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/<dynamic_album_id>")]
pub async fn album_page(dynamic_album_id: String) -> Option<NamedFile> {
    if dynamic_album_id.starts_with("album-") {
        NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
            .await
            .ok()
    } else {
        None
    }
}

#[get("/share/<_path..>")]
pub async fn share(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/archived")]
pub async fn archived() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/archived/view/<_path..>")]
pub async fn archived_view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/trashed")]
pub async fn trashed() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/trashed/view/<_path..>")]
pub async fn trashed_view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/all")]
pub async fn all() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/all/view/<_path..>")]
pub async fn all_view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/videos")]
pub async fn videos() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/videos/view/<_path..>")]
pub async fn videos_view(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/tags")]
pub async fn tags() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/links")]
pub async fn links() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/setting")]
pub async fn setting() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/index.html"))
        .await
        .ok()
}

#[get("/favicon.ico")]
pub async fn favicon() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/favicon.ico"))
        .await
        .ok()
}

#[get("/registerSW.js")]
pub async fn sregister_sw() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/registerSW.js"))
        .await
        .ok()
}

#[get("/serviceWorker.js")]
pub async fn service_worker() -> Option<NamedFile> {
    NamedFile::open(Path::new("../gallery-frontend/dist/serviceWorker.js"))
        .await
        .ok()
}

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

impl From<&AbstractData> for ReducedData {
    fn from(source: &AbstractData) -> Self {
        Self {
            hash: source.hash(),
            width: source.width(),
            height: source.height(),
            date: source.compute_timestamp() as u128,
        }
    }
}

fn check_query_cache(
    query_hash: u64,
    resolved_share_option: &mut Option<ResolvedShare>,
) -> Option<Json<PrefetchReturn>> {
    let find_cache_start_time = Instant::now();

    // Check cache first
    if let Ok(Some(prefetch)) = (&*QUERY_SNAPSHOT).read_query_snapshot(query_hash) {
        let duration = format!("{:?}", find_cache_start_time.elapsed());
        info!(duration = &*duration; "Query cache found");
        let claims = ClaimsTimestamp::new(mem::take(resolved_share_option), prefetch.timestamp);
        return Some(Json(PrefetchReturn::new(
            prefetch,
            claims.encode(),
            claims.resolved_share_opt,
        )));
    }

    let duration = format!("{:?}", find_cache_start_time.elapsed());
    info!(duration = &*duration; "Query cache not found. Generate a new one.");
    None
}

fn filter_items(
    expression_option: Option<Expression>,
    resolved_share_option: &Option<ResolvedShare>,
) -> Result<Vec<ReducedData>> {
    let filter_items_start_time = Instant::now();
    let tree_guard = TREE.in_memory.read().map_err(|err| anyhow!("{:?}", err))?;
    let reduced_data_vector: Vec<ReducedData> = match (expression_option, &resolved_share_option) {
        // If we have a resolved share then it must have a filter expression
        (Some(expr), Some(resolved_share)) => {
            let filter_fn = if resolved_share.share.show_metadata {
                expr.generate_filter()
            } else {
                expr.generate_filter_hide_metadata(resolved_share.album_id)
            };
            tree_guard
                .par_iter()
                .filter(|db_ts| filter_fn(db_ts))
                .map(|db_ts| db_ts.into())
                .collect()
        }
        (Some(expr), None) => {
            let filter_fn = expr.generate_filter();
            tree_guard
                .par_iter()
                .filter(|database_timestamp| filter_fn(database_timestamp))
                .map(|database_timestamp| database_timestamp.into())
                .collect()
        }
        (None, _) => tree_guard
            .par_iter()
            .map(|database_timestamp| database_timestamp.into())
            .collect(),
    };

    let duration = format!("{:?}", filter_items_start_time.elapsed());
    info!(duration = &*duration; "Filter items");

    Ok(reduced_data_vector)
}

fn compute_locate(
    reduced_data_vector: &[ReducedData],
    locate_option: &Option<String>,
) -> Option<usize> {
    let layout_start_time = Instant::now();

    // Find locate index if requested
    let locate_to_index = locate_option.as_ref().and_then(|hash| {
        reduced_data_vector
            .par_iter()
            .position_first(|reduced| reduced.hash.as_str() == hash)
    });

    let duration = format!("{:?}", layout_start_time.elapsed());
    info!(duration = &*duration; "Compute layout");

    locate_to_index
}

fn build_cache_key(expression_option: &Option<Expression>, locate_option: &Option<String>) -> u64 {
    let cache_key_start_time = Instant::now();

    let mut hasher = DefaultHasher::new();
    expression_option.hash(&mut hasher);
    VERSION_COUNT_TIMESTAMP
        .load(Ordering::Relaxed)
        .hash(&mut hasher);
    locate_option.hash(&mut hasher);
    let query_hash = hasher.finish();

    let duration = format!("{:?}", cache_key_start_time.elapsed());
    info!(duration = &*duration; "Build cache key");

    query_hash
}

fn insert_data_into_tree_snapshot(reduced_data_vector: Vec<ReducedData>) -> Result<(u128, usize)> {
    let db_start_time = Instant::now();

    // Persist to snapshot
    let timestamp_millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let reduced_data_vector_length = reduced_data_vector.len();
    TREE_SNAPSHOT
        .in_memory
        .insert(timestamp_millis, reduced_data_vector);
    BATCH_COORDINATOR.execute_batch_detached(FlushTreeSnapshotTask);

    let duration = format!("{:?}", db_start_time.elapsed());
    info!(duration = &*duration; "Write cache into memory");

    Ok((timestamp_millis, reduced_data_vector_length))
}

fn create_json_response(
    timestamp_millis: u128,
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

    let duration = format!("{:?}", json_start_time.elapsed());
    info!(duration = &*duration; "Create JSON response");

    json
}

fn execute_prefetch_logic(
    expression_option: Option<Expression>,
    locate_option: Option<String>,
    mut resolved_share_option: Option<ResolvedShare>,
) -> Result<Json<PrefetchReturn>> {
    // Start timer
    let start_time = Instant::now();

    // Step 1: Build cache key for response creation
    let query_hash = build_cache_key(&expression_option, &locate_option);

    // Step 2: Check if query cache is available
    if let Some(cached_response) = check_query_cache(query_hash, &mut resolved_share_option) {
        return Ok(cached_response);
    }

    // Step 3: Filter items
    let reduced_data_vector = filter_items(expression_option, &resolved_share_option)?;

    // Step 4: Compute layout
    let locate_to_index = compute_locate(&reduced_data_vector, &locate_option);

    // Step 6: Insert data into TREE_SNAPSHOT
    let (timestamp_millis, reduced_data_vector_length) =
        insert_data_into_tree_snapshot(reduced_data_vector)?;

    // Step 7: Create and return JSON response
    let json = create_json_response(
        timestamp_millis,
        locate_to_index,
        reduced_data_vector_length,
        query_hash,
        resolved_share_option,
    );

    // Total elapsed time
    let duration = format!("{:?}", start_time.elapsed());
    info!(duration = &*duration; "(total time) Get_data_length complete");

    Ok(json)
}

#[post("/get/prefetch?<locate>", format = "json", data = "<query_data>")]
pub async fn prefetch(
    auth_guard: GuardResult<GuardShare>,
    query_data: Option<Json<Expression>>,
    locate: Option<String>,
) -> AppResult<Json<PrefetchReturn>> {
    let auth_guard = auth_guard?;

    let client_expr = query_data.map(|wrapper| wrapper.into_inner());
    let resolved_share_option = auth_guard.claims.get_share();

    // Determine the final expression based on user input and share constraints
    let combined_expression = match (client_expr, &resolved_share_option) {
        // Visitor with search query -> Enforce Album ID AND Search Query
        (Some(expr), Some(share)) => Some(Expression::And(vec![
            Expression::Album(share.album_id),
            expr,
        ])),
        // Visitor without search query -> Enforce Album ID
        (None, Some(share)) => Some(Expression::Album(share.album_id)),
        // Owner/Admin -> Use original query (None or Some)
        (expr, None) => expr,
    };

    // Execute on blocking thread
    let job_handle = tokio::task::spawn_blocking(move || {
        execute_prefetch_logic(combined_expression, locate, resolved_share_option)
    })
    .await??;

    Ok(job_handle)
}

#[derive(FromForm, Debug)]
pub struct UploadForm<'r> {
    /// 依序收到的多個檔案
    #[field(name = "file")]
    pub files: Vec<TempFile<'r>>,

    /// 與檔案順序對應的 lastModified 時戳
    #[field(name = "lastModified")]
    pub last_modified: Vec<u64>,
}

fn get_filename(file: &TempFile<'_>) -> String {
    file.name()
        .map(|name| name.to_string())
        .unwrap_or_else(|| "".to_string())
}

#[post("/upload?<presigned_album_id_opt>", data = "<form>")]
pub async fn upload(
    auth: GuardResult<GuardUpload>,
    read_only_mode: Result<GuardReadOnlyMode>,
    presigned_album_id_opt: Option<String>,
    form: Result<Form<UploadForm<'_>>, Errors<'_>>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let mut inner_form = match form {
        Ok(form) => form.into_inner(),
        Err(errors) => {
            let error_chain = errors
                .iter()
                .map(|e| anyhow!(e.to_string()))
                .reduce(|acc, e| acc.context(e.to_string()));

            return match error_chain {
                Some(chain) => Err(chain.context("Failed to parse form").into()),
                None => Err(anyhow!("Failed to parse form with unknown error").into()),
            };
        }
    };

    let presigned_album_id_opt: Option<ArrayString<64>> = if let Some(s) = presigned_album_id_opt {
        Some(
            ArrayString::from(&s)
                .map_err(|_| anyhow!("Failed to create ArrayString from presigned_album_id_opt"))?,
        )
    } else {
        None
    };

    if inner_form.files.len() != inner_form.last_modified.len() {
        return Err(
            anyhow!("Mismatch between number of files and lastModified timestamps.").into(),
        );
    }

    for (i, file) in inner_form.files.iter_mut().enumerate() {
        let last_modified_time = inner_form.last_modified[i];
        let start_time = Instant::now();
        let filename = get_filename(file);
        let extension = get_extension(file)?;

        warn!(duration = &*format!("{:?}", start_time.elapsed());
            "Get file '{}.{}'",
            filename,
            extension,
        );

        if VALID_IMAGE_EXTENSIONS.contains(&extension.as_str())
            || VALID_VIDEO_EXTENSIONS.contains(&extension.as_str())
        {
            let final_path = save_file(file, filename, extension, last_modified_time).await?;
            index_workflow(final_path, presigned_album_id_opt).await?;
        } else {
            error!("Invalid file type");
            return Err(anyhow::anyhow!("Invalid file type: {}", extension).into());
        }
    }

    Ok(())
}

async fn save_file(
    file: &mut TempFile<'_>,
    filename: String,
    extension: String,
    last_modified_time: u64,
) -> Result<String> {
    let unique_id = Uuid::new_v4();
    let path_tmp = format!("./upload/{}-{}.tmp", filename, unique_id);

    file.move_copy_to(&path_tmp).await?;

    let filename = filename.clone(); // Needed because filename is moved in path_tmp

    let path_final = spawn_blocking(move || -> Result<String> {
        let path_final = format!("./upload/{}-{}.{}", filename, unique_id, extension);
        set_last_modified_time(&path_tmp, last_modified_time)?;
        std::fs::rename(&path_tmp, &path_final)?;
        Ok(path_final)
    })
    .await??;

    Ok(path_final)
}
fn set_last_modified_time(path: impl AsRef<Path>, last_modified_time: u64) -> Result<()> {
    let mtime = filetime::FileTime::from_unix_time((last_modified_time / 1000) as i64, 0);
    filetime::set_file_mtime(path, mtime)?;
    Ok(())
}

fn get_extension(file: &TempFile<'_>) -> Result<String> {
    match file.content_type() {
        Some(ct) => match ct.extension() {
            Some(ext) => Ok(ext.as_str().to_lowercase()),
            None => {
                error!("Failed to extract file extension.");
                bail!("Failed to extract file extension.")
            }
        },
        None => {
            error!("Failed to get content type.");
            bail!("Failed to get content type.")
        }
    }
}

#[derive(FromForm, Debug)]
pub struct RegenerateThumbnailForm<'r> {
    /// Hash of the image to regenerate thumbnail for
    #[field(name = "hash")]
    pub hash: String,

    /// Frame file to use for thumbnail generation
    #[field(name = "frame")]
    pub frame: TempFile<'r>,
}

#[put("/put/regenerate-thumbnail-with-frame", data = "<form>")]
pub async fn regenerate_thumbnail_with_frame(
    auth: GuardResult<GuardAuth>,
    read_only_mode: Result<GuardReadOnlyMode>,
    form: Result<Form<RegenerateThumbnailForm<'_>>, Errors<'_>>,
) -> AppResult<()> {
    let _ = auth?;
    let _ = read_only_mode?;
    let mut inner_form = match form {
        Ok(form) => form.into_inner(),
        Err(errors) => {
            let error_chain = errors
                .iter()
                .map(|e| anyhow!(e.to_string()))
                .reduce(|acc, e| acc.context(e.to_string()));

            return match error_chain {
                Some(chain) => Err(chain.context("Failed to parse form").into()),
                None => Err(anyhow!("Failed to parse form with unknown error").into()),
            };
        }
    };

    // Convert hash string to ArrayString
    let hash = ArrayString::<64>::from(&inner_form.hash)
        .map_err(|_| anyhow!("Invalid hash length or format"))?;

    let file_path = thumbnail_path(hash);

    inner_form
        .frame
        .move_copy_to(&file_path)
        .await
        .context("Failed to copy frame file")?;

    let abstract_data = tokio::task::spawn_blocking(move || -> Result<AbstractData> {
        let database_opt = TREE.load_data_from_hash(hash.as_str())?;
        let mut data =
            database_opt.ok_or_else(|| anyhow::anyhow!("Database not found for hash: {}", hash))?;

        let imported_path = match &data {
            AbstractData::Image(i) => imported_path(&i.object.id, &i.metadata.ext),
            AbstractData::Video(v) => imported_path(&v.object.id, &v.metadata.ext),
            _ => return Err(anyhow!("Unsupported type")),
        };

        let index_task =
            crate::background::actors::indexer::IndexTask::new(imported_path, data.clone());

        let dyn_img =
            generate_dynamic_image(&index_task).context("Failed to decode DynamicImage")?;

        let thumbhash = generate_thumbhash(&dyn_img);
        let phash = generate_phash(&dyn_img);

        match &mut data {
            AbstractData::Image(i) => {
                i.object.thumbhash = Some(thumbhash);
                i.metadata.phash = Some(phash);
            }
            AbstractData::Video(v) => {
                v.object.thumbhash = Some(thumbhash);
                // Video 沒有 phash
            }
            _ => {}
        }

        Ok(data)
    })
    .await
    .context("Failed to spawn blocking task")??;

    INDEX_COORDINATOR
        .execute_batch_waiting(FlushTreeTask::insert(vec![abstract_data]))
        .await
        .context("Failed to execute FlushTreeTask")?;

    BATCH_COORDINATOR
        .execute_batch_waiting(UpdateTreeTask)
        .await
        .unwrap();

    info!("Regenerating thumbnail successfully");
    Ok(())
}

pub fn generate_media_routes() -> Vec<rocket::Route> {
    routes![
        compressed_file,
        imported_file,
        get_config,
        get_tags,
        get_albums,
        get_all_shares,
        get_data,
        get_rows,
        get_scroll_bar,
        redirect_to_photo,
        login,
        redirect_to_login,
        unauthorized,
        home,
        home_view,
        favorite,
        favorite_view,
        albums,
        albums_view,
        album_page,
        share,
        archived,
        archived_view,
        trashed,
        trashed_view,
        all,
        all_view,
        videos,
        videos_view,
        tags,
        links,
        setting,
        favicon,
        sregister_sw,
        service_worker,
        prefetch,
        upload,
        regenerate_thumbnail_with_frame
    ]
}
