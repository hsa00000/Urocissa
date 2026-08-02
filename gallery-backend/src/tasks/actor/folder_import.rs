use chrono::Utc;
use log::{info, warn};
use mini_executor::Task;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio::task::JoinHandle;
use walkdir::{DirEntry, WalkDir};

use crate::{
    public::{
        constant::storage::get_data_path,
        error::{AppError, ErrorKind},
        error_data::handle_error,
        media::is_valid_media_file,
    },
    router::AppResult,
    tasks::INDEX_COORDINATOR,
    workflow::index_for_watch,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FolderImportState {
    Idle,
    Running,
    Completed,
    Canceled,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderImportStatus {
    pub state: FolderImportState,
    pub root: Option<String>,
    pub scanned: u64,
    pub matched: u64,
    pub processed: u64,
    pub failed: u64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub cancel_requested: bool,
}

impl Default for FolderImportStatus {
    fn default() -> Self {
        Self {
            state: FolderImportState::Idle,
            root: None,
            scanned: 0,
            matched: 0,
            processed: 0,
            failed: 0,
            started_at: None,
            finished_at: None,
            cancel_requested: false,
        }
    }
}

#[derive(Clone)]
struct ActiveImport {
    job_id: u64,
    cancel: Arc<AtomicBool>,
}

struct ImportStatusSlot {
    job_id: u64,
    status: FolderImportStatus,
}

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);
static IMPORT_STATUS: LazyLock<Mutex<ImportStatusSlot>> = LazyLock::new(|| {
    Mutex::new(ImportStatusSlot {
        job_id: 0,
        status: FolderImportStatus::default(),
    })
});
static ACTIVE_IMPORT: LazyLock<Mutex<Option<ActiveImport>>> = LazyLock::new(|| Mutex::new(None));

pub fn folder_import_status() -> FolderImportStatus {
    IMPORT_STATUS.lock().unwrap().status.clone()
}

pub fn start_folder_import(path: &str) -> AppResult<()> {
    let root = canonicalize_import_root(path)?;
    let internal_roots = internal_subtree_roots();

    if is_inside_internal_subtree(&root, &internal_roots) {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "Cannot import Urocissa internal data directories",
        ));
    }

    let job_id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));

    {
        let mut slot = IMPORT_STATUS.lock().unwrap();
        if slot.status.state == FolderImportState::Running {
            return Err(AppError::new(
                ErrorKind::Conflict,
                "A folder import job is already running",
            ));
        }

        slot.job_id = job_id;
        slot.status = FolderImportStatus {
            state: FolderImportState::Running,
            root: Some(root.to_string_lossy().into_owned()),
            scanned: 0,
            matched: 0,
            processed: 0,
            failed: 0,
            started_at: Some(Utc::now().timestamp_millis()),
            finished_at: None,
            cancel_requested: false,
        };
    }

    *ACTIVE_IMPORT.lock().unwrap() = Some(ActiveImport {
        job_id,
        cancel: cancel.clone(),
    });

    INDEX_COORDINATOR.execute_detached(FolderImportTask::new(job_id, root, internal_roots, cancel));

    Ok(())
}

pub fn cancel_folder_import() -> AppResult<()> {
    let active = ACTIVE_IMPORT.lock().unwrap().clone();
    let Some(active) = active else {
        return Err(AppError::new(
            ErrorKind::NotFound,
            "No active folder import job",
        ));
    };

    active.cancel.store(true, Ordering::SeqCst);
    let mut slot = IMPORT_STATUS.lock().unwrap();
    if slot.job_id == active.job_id && slot.status.state == FolderImportState::Running {
        slot.status.cancel_requested = true;
    }

    Ok(())
}

fn canonicalize_import_root(path: &str) -> AppResult<PathBuf> {
    let trimmed = path.trim().trim_matches('"');
    if trimmed.is_empty() {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "Import path cannot be empty",
        ));
    }

    let root = fs::canonicalize(PathBuf::from(trimmed)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::new(ErrorKind::NotFound, "Directory not found")
        } else {
            AppError::from_err(ErrorKind::IO, e.into())
        }
    })?;

    if !root.is_dir() {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "Import path must be a directory",
        ));
    }

    Ok(root)
}

fn internal_subtree_roots() -> Vec<PathBuf> {
    let data_path = get_data_path();
    let data_root = fs::canonicalize(data_path).unwrap_or_else(|_| {
        if data_path.is_absolute() {
            data_path.clone()
        } else {
            std::env::current_dir().unwrap_or_default().join(data_path)
        }
    });

    ["object", "db", "upload"]
        .into_iter()
        .map(|name| {
            let path = data_root.join(name);
            fs::canonicalize(&path).unwrap_or(path)
        })
        .collect()
}

fn is_inside_internal_subtree(path: &Path, internal_roots: &[PathBuf]) -> bool {
    internal_roots
        .iter()
        .any(|internal| path == internal || path.starts_with(internal))
}

fn should_walk_entry(entry: &DirEntry, internal_roots: &[PathBuf]) -> bool {
    entry.depth() == 0 || !is_inside_internal_subtree(entry.path(), internal_roots)
}

fn update_status(job_id: u64, update: impl FnOnce(&mut FolderImportStatus)) {
    let mut slot = IMPORT_STATUS.lock().unwrap();
    if slot.job_id == job_id {
        update(&mut slot.status);
    }
}

fn increment_scanned(job_id: u64) {
    update_status(job_id, |status| status.scanned += 1);
}

fn increment_matched(job_id: u64) {
    update_status(job_id, |status| status.matched += 1);
}

fn increment_processed(job_id: u64) {
    update_status(job_id, |status| status.processed += 1);
}

fn increment_failed(job_id: u64) {
    update_status(job_id, |status| status.failed += 1);
}

fn did_every_matched_file_fail(job_id: u64) -> bool {
    let slot = IMPORT_STATUS.lock().unwrap();
    slot.job_id == job_id
        && slot.status.matched > 0
        && slot.status.processed == 0
        && slot.status.failed >= slot.status.matched
}

fn finish_job(job_id: u64, state: FolderImportState) {
    {
        let mut slot = IMPORT_STATUS.lock().unwrap();
        if slot.job_id == job_id {
            slot.status.state = state;
            slot.status.finished_at = Some(Utc::now().timestamp_millis());
        }
    }

    let mut active = ACTIVE_IMPORT.lock().unwrap();
    if active.as_ref().is_some_and(|job| job.job_id == job_id) {
        *active = None;
    }
}

pub struct FolderImportTask {
    job_id: u64,
    root: PathBuf,
    internal_roots: Vec<PathBuf>,
    cancel: Arc<AtomicBool>,
}

impl FolderImportTask {
    fn new(
        job_id: u64,
        root: PathBuf,
        internal_roots: Vec<PathBuf>,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        Self {
            job_id,
            root,
            internal_roots,
            cancel,
        }
    }
}

impl Task for FolderImportTask {
    type Output = ();

    async fn run(self) -> Self::Output {
        info!("Starting one-time folder import: {}", self.root.display());

        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let walker = WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| should_walk_entry(entry, &self.internal_roots));

        for entry in walker {
            if self.cancel.load(Ordering::SeqCst) {
                break;
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    warn!("Folder import walk error: {err}");
                    increment_failed(self.job_id);
                    continue;
                }
            };

            if !entry.file_type().is_file() {
                continue;
            }

            increment_scanned(self.job_id);

            let path = entry.into_path();
            if !is_valid_media_file(&path) {
                continue;
            }

            increment_matched(self.job_id);
            handles.push(
                INDEX_COORDINATOR.execute_detached(FolderImportFileTask::new(self.job_id, path)),
            );
        }

        for handle in handles {
            if let Err(err) = handle.await {
                warn!("Folder import file task join error: {err}");
                increment_failed(self.job_id);
            }
        }

        let state = if self.cancel.load(Ordering::SeqCst) {
            FolderImportState::Canceled
        } else if did_every_matched_file_fail(self.job_id) {
            FolderImportState::Failed
        } else {
            FolderImportState::Completed
        };

        finish_job(self.job_id, state);
    }
}

pub struct FolderImportFileTask {
    job_id: u64,
    path: PathBuf,
}

impl FolderImportFileTask {
    fn new(job_id: u64, path: PathBuf) -> Self {
        Self { job_id, path }
    }
}

impl Task for FolderImportFileTask {
    type Output = ();

    async fn run(self) -> Self::Output {
        match index_for_watch(self.path, None).await {
            Ok(()) => increment_processed(self.job_id),
            Err(err) => {
                handle_error(err);
                increment_failed(self.job_id);
            }
        }
    }
}
