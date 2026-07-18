#[macro_use]
extern crate rocket;
use redb::{ReadableTable, ReadableTableMetadata};
use std::thread;
use std::time::Instant;

mod operations;
mod performance;
mod process;
mod public;
mod router;
mod tasks;
mod workflow;

use crate::operations::initialization::logger::initialize_logger;
use crate::process::initialization::initialize;
use crate::public::constant::runtime::{INDEX_RUNTIME, ROCKET_RUNTIME};
use crate::public::constant::storage::get_data_path;

use crate::public::error_data::handle_error;
use crate::public::tui::{DASHBOARD, tui_task};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::start_watcher::StartWatcherTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::tasks::looper::start_expire_check_loop;
use public::constant::redb::DATA_TABLE;
use public::db::tree::TREE;
use public::structure::abstract_data::AbstractData;

fn migration() {
    let v4_db_path = get_data_path().join("db/index_v4.redb");

    if v4_db_path.exists() {
        eprintln!(
            "Old database format detected at: {}\n\n\
             This version cannot directly migrate from database v4.\n\
             Please follow these steps to upgrade safely:\n\
             1. Downgrade Urocissa to version 1.2.2.\n\
             2. Start the app (v1.2.2) to automatically migrate the database.\n\
             3. Once confirmed working, update back to this latest version.\n\n\
             Press Enter to exit...",
            v4_db_path.display()
        );

        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        std::process::exit(1);
    }
}

fn main() {
    // Initialize logger first thing
    let tui_events_rx = initialize_logger();
    performance::initialize();

    migration();

    // Initialize core subsystems (Config, DB, FFmpeg checks)
    initialize();

    #[cfg(feature = "embed-frontend")]
    info!("Frontend Configuration: EMBEDDED (Assets compiled into binary)");
    #[cfg(not(feature = "embed-frontend"))]
    info!("Frontend Configuration: EXTERNAL (Loading from file system)");

    // Architecture: Isolate the Indexing/TUI runtime from the Rocket server runtime.

    // This prevents heavy blocking operations in the indexer from stalling web requests.
    let worker_handle = thread::spawn(move || {
        INDEX_RUNTIME.block_on(async {
            let start_time = Instant::now();
            let txn = TREE.in_disk.begin_write().unwrap();

            {
                let table = txn.open_table(DATA_TABLE).unwrap();
                let total_count = table.len().unwrap();

                // Constraint: DATA_TABLE stores mixed types (Albums and Media).
                // We must perform an O(N) scan to differentiate counts.
                let album_count = table
                    .iter()
                    .unwrap()
                    .filter_map(std::result::Result::ok)
                    .filter(|(_, guard)| matches!(guard.value(), AbstractData::Album(_)))
                    .count();

                let media_count = usize::try_from(total_count).unwrap_or(0) - album_count;

                crate::perf_timing!(
                    "startup.read_database_count",
                    start_time,
                    "Read {} photos/videos and {} albums from database.",
                    media_count,
                    album_count
                );
            }

            txn.commit().unwrap();

            BATCH_COORDINATOR.execute_batch_detached(StartWatcherTask);
            BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
            start_expire_check_loop();

            if let Some(console) = superconsole::SuperConsole::new() {
                INDEX_RUNTIME.spawn(async move {
                    if let Err(e) = tui_task(console, DASHBOARD.clone(), tui_events_rx)
                        .await
                        .map_err(|error| handle_error(error.context("TUI error.")))
                    {
                        panic!("TUI error: {e:?}");
                    }
                });
            } else {
                error!("Superconsole disabled (no TTY)");
            }

            if let Err(e) = tokio::signal::ctrl_c().await {
                error!("Failed to listen for ctrl-c in worker: {}", e);
            }
            info!("Worker thread shutting down.");
        });
    });

    let rocket_handle = thread::spawn(|| {
        info!("Rocket thread starting.");
        if let Err(e) = ROCKET_RUNTIME.block_on(async {
            let rocket = router::build_rocket().ignite().await?;
            #[cfg(feature = "auto-open-browser")]
            let port = rocket.config().port;
            let shutdown_handle = rocket.shutdown();

            // Manually handle Ctrl-C to trigger graceful shutdown
            // since we are running outside the default global runtime.
            ROCKET_RUNTIME.spawn(async move {
                let _ = tokio::signal::ctrl_c().await;
                info!("Ctrl-C received, shutting down Rocket server gracefully.");
                shutdown_handle.notify();
            });

            // Open browser after server starts listening
            let launch_future = rocket.launch();
            #[cfg(feature = "auto-open-browser")]
            open_browser(port);
            launch_future.await.map_err(anyhow::Error::from)
        }) {
            error!("Rocket thread exited with an error: {}", e);
        }
    });

    worker_handle.join().expect("Worker thread panicked");
    rocket_handle.join().expect("Rocket thread panicked");
}

#[cfg(feature = "auto-open-browser")]
fn open_browser(port: u16) {
    let url = format!("http://localhost:{}", port);
    info!("Opening browser at {}", url);
    if let Err(e) = webbrowser::open(&url) {
        error!("Failed to open browser: {}", e);
    }
}
