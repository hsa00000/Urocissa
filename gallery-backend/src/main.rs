#[macro_use]
extern crate rocket;
use std::{sync::mpsc::sync_channel, thread, time::Instant};

mod operations;
mod performance;
mod process;
mod public;
mod router;
mod storage;
mod tasks;
mod workflow;

use crate::operations::initialization::logger::initialize_logger;
use crate::process::initialization::initialize;
use crate::public::constant::runtime::{INDEX_RUNTIME, ROCKET_RUNTIME};
use crate::public::error_data::handle_error;
use crate::public::tui::{DASHBOARD, tui_task};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::start_watcher::StartWatcherTask;
use crate::tasks::batcher::update_tree::update_tree_task;
use crate::tasks::looper::start_expire_check_loop;
use public::db::tree::TREE;
use public::structure::abstract_data::AbstractData;
use storage::migration::prepare_storage;

fn main() {
    // Initialize logger first thing
    let tui_events_rx = initialize_logger();
    performance::initialize();

    if let Err(error) = prepare_storage() {
        eprintln!("Database preparation failed: {error:#}");
        std::process::exit(1);
    }

    // Initialize core subsystems (Config, DB, FFmpeg checks)
    initialize();

    #[cfg(feature = "embed-frontend")]
    info!("Frontend Configuration: EMBEDDED (Assets compiled into binary)");
    #[cfg(not(feature = "embed-frontend"))]
    info!("Frontend Configuration: EXTERNAL (Loading from file system)");

    // Architecture: Isolate the Indexing/TUI runtime from the Rocket server runtime.

    // This prevents heavy blocking operations in the indexer from stalling web requests.
    let (tree_ready_tx, tree_ready_rx) = sync_channel(1);
    let worker_handle = thread::spawn(move || {
        INDEX_RUNTIME.block_on(async {
            let start_time = Instant::now();
            let (total_count, album_count) = TREE
                .store
                .read(|table| {
                    let total_count = table.len()?;
                    let album_count = table.iter()?.try_fold(0_usize, |count, entry| {
                        let (_, value) = entry?;
                        Ok::<usize, anyhow::Error>(
                            count + usize::from(matches!(value.value(), AbstractData::Album(_))),
                        )
                    })?;
                    Ok::<(u64, usize), anyhow::Error>((total_count, album_count))
                })
                .unwrap();

            let media_count = usize::try_from(total_count).unwrap_or(0) - album_count;

            crate::perf_timing!(
                "startup.read_database_count",
                start_time,
                "Read {} photos/videos and {} albums from database.",
                media_count,
                album_count
            );

            // Build the first in-memory tree and list snapshot before Rocket
            // starts accepting requests. Later rebuilds remain asynchronous.
            update_tree_task();
            if tree_ready_tx.send(()).is_err() {
                error!("Failed to signal initial tree readiness.");
                return;
            }

            BATCH_COORDINATOR.execute_batch_detached(StartWatcherTask);
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

    if tree_ready_rx.recv().is_err() {
        eprintln!("Initial tree preparation failed; shutting down.");
        std::process::exit(1);
    }

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
