#[macro_use]
extern crate rocket;
use anyhow::Result;

mod public;
mod router;
mod table;
mod workflow;

use crate::public::constant::runtime::{INDEX_RUNTIME, ROCKET_RUNTIME};
use crate::public::error_data::handle_error;
use crate::public::tui::{DASHBOARD, tui_task};
use crate::workflow::processors::setup::initialize;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::start_watcher::StartWatcherTask;
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;

use rocket::fs::FileServer;
use router::fairing::cache_control_fairing::cache_control_fairing;
use router::fairing::generate_fairing_routes;
use router::{
    delete::generate_delete_routes, get::generate_get_routes, post::generate_post_routes,
    put::generate_put_routes,
};
use std::thread;
use std::time::Instant;
use tokio::sync::broadcast;

async fn build_rocket() -> rocket::Rocket<rocket::Build> {
    // 1. 建立設定：禁用 Rocket 內建的 Ctrl-C 監聽
    let figment = rocket::Config::figment()
        .merge(("shutdown.ctrlc", false));

    // 2. 使用 custom(figment) 取代 build()
    rocket::custom(figment)
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
    crate::public::db::sqlite::init_db_file_once()?;

    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    let worker_handle = thread::spawn({
        let shutdown_tx = shutdown_tx.clone();
        move || {
            INDEX_RUNTIME.block_on(async {
                let rx = initialize();
                let start_time = Instant::now();
                let conn = crate::public::db::tree::TREE.get_connection().expect("Failed to get DB connection");
                let data_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM object WHERE obj_type IN ('image', 'video')", 
                    [], 
                    |row| row.get(0)
                ).expect("Failed to count data");
                info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} photos/videos from database.", data_count);
                let album_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM object WHERE obj_type = 'album'", 
                    [], 
                    |row| row.get(0)
                ).expect("Failed to count albums");
                info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} albums from database.", album_count);
                BATCH_COORDINATOR.execute_batch_detached(StartWatcherTask);
                BATCH_COORDINATOR.execute_batch_detached(UpdateTreeTask);

                let mut tui_handle = None;

                if let Some(sc) = superconsole::SuperConsole::new() {
                    let shutdown_tx_clone = shutdown_tx.clone();
                    tui_handle = Some(INDEX_RUNTIME.spawn(async move {
                        if let Err(e) = tui_task(sc, DASHBOARD.clone(), rx)
                            .await
                            .map_err(|error|handle_error(error.context("TUI error.")))
                        {
                            error!("TUI error: {e:?}");
                            let _ = shutdown_tx_clone.send(());
                        }
                    }));
                } else {
                    error!("Superconsole disabled (no TTY)");
                }

                let mut shutdown_rx = shutdown_tx.subscribe();
                
                // 3. 判斷是誰觸發了關閉
                let is_ctrl_c = tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        // 【測試點 1】確認真的有收到信號（直接用 println!）
                        println!("\n[DEBUG] Ctrl-C signal detected in worker!"); 
                        true
                    },
                    _ = shutdown_rx.recv() => {
                        println!("\n[DEBUG] Internal shutdown signal detected!");
                        false
                    },
                };

                // 4. 先停止 TUI
                if let Some(handle) = tui_handle {
                    handle.abort();
                    let _ = handle.await; // 等待任務結束
                }

                // 【修正方案】移除 sleep，改用 flush
                // 這會強制將 SuperConsole 留下的 "還原終端機" 指令立刻送出
                let _ = std::io::stdout().flush();

                // 5. 再次通知與印出 Log
                // 為了雙重保險，我們在字串前面加 \r (回到行首) 和 \x1b[2K (清除整行)
                // 這樣就算 TUI 殘留了一些髒東西，這行字也能乾淨地印出來
                if is_ctrl_c {
                    println!("\r\x1b[2K[DEBUG] TUI stopped. Notifying Rocket to shutdown...");
                    let _ = shutdown_tx.send(());
                } else {
                    println!("\r\x1b[2K[DEBUG] TUI stopped. Shutdown in progress...");
                }

                println!("Worker thread shutting down successfully.");
            });
        }
    });

    let rocket_handle = thread::spawn({
        let shutdown_tx = shutdown_tx.clone();
        move || {
            info!("Rocket thread starting.");
            let result = ROCKET_RUNTIME.block_on(async {
                let rocket_instance = build_rocket().await.ignite().await?;
                let shutdown_handle = rocket_instance.shutdown();
                let shutdown_tx_clone = shutdown_tx.clone();
                ROCKET_RUNTIME.spawn(async move {
                    let mut shutdown_rx = shutdown_tx_clone.subscribe();
                    
                    // 6. Rocket 執行緒不再自己聽 Ctrl-C，只等待 worker 發出的廣播
                    if shutdown_rx.recv().await.is_ok() {
                        info!("Shutdown signal received, shutting down Rocket server gracefully.");
                        shutdown_handle.notify();
                    }
                });
                rocket_instance.launch().await
            });
            if let Err(e) = result {
                error!("Rocket server failed: {}", e);
                let _ = shutdown_tx.send(());
                return Err(anyhow::Error::from(e));
            }
            Ok(())
        }
    });

    worker_handle.join().expect("Worker thread panicked");
    rocket_handle.join().expect("Rocket thread panicked");

    Ok(())
}
