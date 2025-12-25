#[macro_use]
extern crate rocket;
use anyhow::Result;
// --- Make sure all your modules are declared ---

mod operations;
mod process;
mod public;
mod router;
mod tasks;
mod workflow;

use crate::process::initialization::initialize;
use crate::public::constant::runtime::{INDEX_RUNTIME, ROCKET_RUNTIME};
use crate::public::error_data::handle_error;
use crate::public::tui::{DASHBOARD, tui_task};
use crate::tasks::BATCH_COORDINATOR;
use crate::tasks::batcher::start_watcher::StartWatcherTask;
use crate::tasks::batcher::update_tree::UpdateTreeTask;
use crate::tasks::looper::start_expire_check_loop;

use public::constant::redb::DATA_TABLE;
use public::db::tree::TREE;
use public::structure::abstract_data::AbstractData;
use redb::{ReadableTable, ReadableTableMetadata};
use rocket::fs::FileServer;
use router::fairing::cache_control_fairing::cache_control_fairing;
use router::fairing::generate_fairing_routes;
use router::{
    delete::generate_delete_routes, get::generate_get_routes, post::generate_post_routes,
    put::generate_put_routes,
};
use std::thread;
use std::time::Instant;

async fn build_rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .attach(cache_control_fairing())
        .mount(
            "/assets",
            FileServer::from("../gallery-frontend/dist/assets"),
        )
        .mount("/", generate_get_routes())
        .mount("/", generate_post_routes())
        .mount("/", generate_put_routes())
        .mount("/", generate_delete_routes())
        .mount("/", generate_fairing_routes())
}

fn main() -> Result<()> {
    let worker_handle = thread::spawn(|| {
        INDEX_RUNTIME.block_on(async {
            let rx = initialize();
            let start_time = Instant::now();
            let txn = TREE.in_disk.begin_write().unwrap();
            {
                let table = txn.open_table(DATA_TABLE).unwrap();
                let total_count = table.len().unwrap();
                let album_count = table.iter().unwrap()
                    .filter(|entry| {
                        if let Ok((_, guard)) = entry {
                            matches!(guard.value(), AbstractData::Album(_))
                        } else {
                            false
                        }
                    })
                    .count();
                let media_count = total_count as usize - album_count;
                info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} photos/videos and {} albums from database.", media_count, album_count);
            }
            txn.commit().unwrap();
            BATCH_COORDINATOR.execute_batch_detached(StartWatcherTask);
            BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);
            start_expire_check_loop();

            if let Some(sc) = superconsole::SuperConsole::new() {
                INDEX_RUNTIME.spawn(async move {
                    if let Err(e) = tui_task(sc, DASHBOARD.clone(), rx)
                        .await
                        .map_err(|error|handle_error(error.context("TUI error.")))
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
            let rocket_instance = build_rocket().await.ignite().await?;
            let shutdown_handle = rocket_instance.shutdown();
            ROCKET_RUNTIME.spawn(async move {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to install Ctrl-C handler for Rocket");
                info!("Ctrl-C received, shutting down Rocket server gracefully.");
                shutdown_handle.notify();
            });
            if let Err(e) = rocket_instance.launch().await {
                error!("Rocket server failed: {}", e);
                return Err(anyhow::Error::from(e));
            }
            Ok(())
        }) {
            error!("Rocket thread exited with an error: {}", e);
        }
    });

    worker_handle.join().expect("Worker thread panicked");
    rocket_handle.join().expect("Rocket thread panicked");

    Ok(())
}
