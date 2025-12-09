#[macro_use]
extern crate rocket;
use anyhow::Result;

mod public;
mod router;
mod table;
mod utils;
mod workflow;

use crate::public::constant::runtime::{INDEX_RUNTIME, ROCKET_RUNTIME};
use crate::public::error_data::handle_error;
use crate::public::tui::{DASHBOARD, tui_task};
use crate::workflow::processors::setup::initialize;
use crate::workflow::tasks::BATCH_COORDINATOR;
use crate::workflow::tasks::batcher::start_watcher::StartWatcherTask;
use crate::workflow::tasks::batcher::update_tree::UpdateTreeTask;

use redb::ReadableTable;
use rocket::fs::FileServer;
use router::fairing::cache_control_fairing::cache_control_fairing;
use router::fairing::generate_fairing_routes;
use router::{
    delete::generate_delete_routes, get::generate_get_routes, post::generate_post_routes,
    put::generate_put_routes,
};
use std::io::Write;
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
    // Database initialization is now lazy
    
    // [新增] 在啟動任何執行緒前，先初始化所有資料表
    // 這樣可以確保後續的讀取交易不會因為找不到資料表而 Panic
    {
        info!("Initializing database tables...");
        let txn = crate::public::db::tree::TREE.begin_write()?;
        
        // 1. 主要物件與 Metadata 表
        let _ = txn.open_table(crate::table::object::OBJECT_TABLE)?;
        let _ = txn.open_table(crate::table::meta_album::META_ALBUM_TABLE)?;
        let _ = txn.open_table(crate::table::meta_image::META_IMAGE_TABLE)?;
        let _ = txn.open_table(crate::table::meta_video::META_VIDEO_TABLE)?;
        
        // 2. 關聯表 (Relations)
        let _ = txn.open_table(crate::table::relations::album_database::ALBUM_ITEMS_TABLE)?;
        let _ = txn.open_table(crate::table::relations::album_database::ITEM_ALBUMS_TABLE)?;
        let _ = txn.open_table(crate::table::relations::album_share::ALBUM_SHARE_TABLE)?;
        let _ = txn.open_table(crate::table::relations::database_alias::DATABASE_ALIAS_TABLE)?;
        let _ = txn.open_table(crate::table::relations::database_exif::DATABASE_EXIF_TABLE)?;
        let _ = txn.open_table(crate::table::relations::tag_database::TAG_DATABASE_TABLE)?;
        let _ = txn.open_table(crate::table::relations::tag_database::IDX_TAG_HASH_TABLE)?;
        
        txn.commit()?;
        info!("Database tables initialized successfully.");
    }
    
    let (shutdown_tx, _) = broadcast::channel::<()>(1);    let worker_handle = thread::spawn({
        let shutdown_tx = shutdown_tx.clone();
        move || {
            INDEX_RUNTIME.block_on(async {
                let rx = initialize();
                let start_time = Instant::now();
                let txn = crate::public::db::tree::TREE.begin_read().expect("Failed to begin read transaction");
                let object_table = txn.open_table(crate::table::object::OBJECT_TABLE).expect("Failed to open object table");
                
                let mut data_count = 0i64;
                let mut album_count = 0i64;
                
                for item in object_table.iter().unwrap() {
                    let (_, value) = item.unwrap();
                    let object: crate::table::object::ObjectSchema = bitcode::decode(value.value()).unwrap();
                    match object.obj_type {
                        crate::table::object::ObjectType::Image | crate::table::object::ObjectType::Video => data_count += 1,
                        crate::table::object::ObjectType::Album => album_count += 1,
                    }
                }
                
                info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} photos/videos from database.", data_count);
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
    let _ = rocket_handle.join().expect("Rocket thread panicked");

    Ok(())
}
