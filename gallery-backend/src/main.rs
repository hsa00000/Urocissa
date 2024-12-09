#[macro_use]
extern crate rocket;
use initialization::{
    check_ffmpeg_and_ffprobe, initialize_file, initialize_folder, initialize_logger,
};
use public::redb::{ALBUM_TABLE, DATA_TABLE};
use public::tree::start_loop::SHOULD_RESET;
use public::tree::TREE;
use redb::ReadableTableMetadata;
use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use router::fairing::cache_control_fairing;
use router::{
    delete::generate_delete_routes, get::generate_get_routes, post::generate_post_routes,
    put::generate_put_routes,
};
use std::thread;
use std::time::Instant;
mod executor;
mod initialization;
mod public;
mod router;
mod synchronizer;

#[launch]
async fn rocket() -> _ {
    initialize_logger();
    check_ffmpeg_and_ffprobe();
    initialize_folder();
    initialize_file();
    let start_time = Instant::now();
    let txn = TREE.in_disk.begin_write().unwrap();

    {
        let table = txn.open_table(DATA_TABLE).unwrap();
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} photos/vidoes from database.", table.len().unwrap());
        let album_table = txn.open_table(ALBUM_TABLE).unwrap();
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} albums from database.", album_table.len().unwrap());
    }

    txn.commit().unwrap();

    SHOULD_RESET.notify_one();

    rocket::build()
        .attach(cache_control_fairing())
        /* .attach(auth_request_fairing()) */
        .attach(AdHoc::on_liftoff("Shutdown", |rocket| {
            Box::pin(async move {
                let shutdown = rocket.shutdown();
                // dedicated thread and tokio runtime for channel
                thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(synchronizer::start_sync(shutdown))
                });
            })
        }))
        .mount(
            "/assets",
            FileServer::from("../gallery-frontend/dist/assets"),
        )
        .mount("/", generate_get_routes())
        .mount("/", generate_post_routes())
        .mount("/", generate_put_routes())
        .mount("/", generate_delete_routes())
}
