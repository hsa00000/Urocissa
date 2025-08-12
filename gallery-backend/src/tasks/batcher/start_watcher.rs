use crate::public::constant::runtime::INDEX_RUNTIME;
use crate::public::constant::{VALID_IMAGE_EXTENSIONS, VALID_VIDEO_EXTENSIONS};
use crate::{public::config::PRIVATE_CONFIG, workflow::index_for_watch};
use log::{error, info};
use mini_executor::BatchTask;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::time::{Duration, sleep};
use walkdir::WalkDir;

static IS_WATCHING: AtomicBool = AtomicBool::new(false);

static WATCHER_HANDLE: LazyLock<Mutex<Option<RecommendedWatcher>>> =
    LazyLock::new(|| Mutex::new(None));

/// The last trigger time for each path
static DEBOUNCE_POOL: LazyLock<Mutex<HashMap<PathBuf, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct StartWatcherTask;

impl BatchTask for StartWatcherTask {
    fn batch_run(_: Vec<Self>) -> impl Future<Output = ()> + Send {
        async move {
            start_watcher_task();
        }
    }
}

fn start_watcher_task() -> () {
    // Fast-path: already running.
    if IS_WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }

    // Build the watcher.
    let mut watcher = new_watcher().unwrap();
    for path in &PRIVATE_CONFIG.sync_path {
        watcher.watch(path, RecursiveMode::Recursive).unwrap();
        info!("Watching path {:?}", path);
    }

    // Store it globally to keep it alive.
    *WATCHER_HANDLE.lock().unwrap() = Some(watcher);
}

fn is_valid_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .map(|ext| {
            VALID_IMAGE_EXTENSIONS.contains(&ext.as_str())
                || VALID_VIDEO_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

/// Push the path into the debounce pool: if there is no later event for the same path within 1 second, trigger indexing
fn submit_to_debounce_pool(path: PathBuf) {
    let now = Instant::now();

    {
        let mut pool = DEBOUNCE_POOL.lock().unwrap();
        pool.insert(path.clone(), now);
    }

    // Start a task to check after 1 second (running on INDEX_RUNTIME)
    INDEX_RUNTIME.spawn(async move {
        sleep(Duration::from_secs(1)).await;

        // Check if there are any events for the same path within this 1 second (i.e., whether the last time is still now)
        let should_run = {
            let mut pool = DEBOUNCE_POOL.lock().unwrap();
            match pool.get(&path).copied() {
                Some(last) if last == now => {
                    // Not updated, remove and execute
                    pool.remove(&path);
                    true
                }
                _ => false, // There are later events or it has been removed, abandon this time
            }
        };

        if should_run && is_valid_media_file(&path) {
            // Really need to do indexing
            index_for_watch(path, None).await;
        }
    });
}

fn new_watcher() -> notify::Result<RecommendedWatcher> {
    notify::recommended_watcher(move |result: Result<Event, notify::Error>| match result {
        Ok(event) => {
            match event.kind {
                EventKind::Create(_) => {
                    let mut path_list: HashSet<PathBuf> = HashSet::new();

                    for path in event.paths {
                        if path.is_file() {
                            path_list.insert(path);
                        } else if path.is_dir() {
                            WalkDir::new(&path)
                                .into_iter()
                                .filter_map(|dir_entry| dir_entry.ok())
                                .filter(|dir_entry| dir_entry.file_type().is_file())
                                .for_each(|dir_entry| {
                                    path_list.insert(dir_entry.into_path());
                                });
                        }
                    }

                    for path in path_list {
                        if is_valid_media_file(&path) {
                            submit_to_debounce_pool(path);
                        }
                    }
                }

                EventKind::Modify(_) => {
                    let mut path_list: HashSet<PathBuf> = HashSet::new();

                    for path in event.paths {
                        if path.is_file() {
                            path_list.insert(path);
                        }
                    }

                    for path in path_list {
                        if is_valid_media_file(&path) {
                            submit_to_debounce_pool(path);
                        }
                    }
                }

                _ => { /* ignore other kinds */ }
            }
        }
        Err(err) => error!("Watch error: {:#?}", err),
    })
}
