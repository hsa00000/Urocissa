#[macro_use]
extern crate rocket;
use initialization::{initialize_file, initialize_folder, initialize_logger};
use public::redb::{ALBUM_TABLE, DATA_TABLE};
use public::tree::start_loop::SHOULD_RESET;
use public::tree::TREE;
use redb::ReadableTableMetadata;
use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use router::fairing::{auth_request_fairing, cache_control_fairing};
use router::{
    delete::delete_data::delete_data,
    get::get_data::{
        get_albums, get_config, get_data, get_rows, get_scroll_bar, get_tags, prefetch,
    },
    get::get_img::compressed_file,
    get::get_page::{
        album_page, albums, albums_view, all, all_view, archived, archived_view, catch_view_routes,
        favicon, favorite, favorite_view, login, redirect_to_login, redirect_to_photo,
        redirect_to_photo_2, setting, tags, trashed, trashed_view, unauthorized,
    },
    post::{
        authenticate::authenticate, authenticate::authenticate_share, create_album::create_album,
        post_upload::upload,
    },
    put::{
        edit_album::{edit_album, set_album_cover, set_album_title},
        edit_tag::edit_tag,
        random::generate_random_data,
        regenerate_preview::regenerate_preview,
    },
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
    initialize_folder();
    initialize_file();
    let start_time = Instant::now();
    let txn = TREE.in_disk.begin_write().unwrap();

    {
        println!("test");
        let table = txn.open_table(DATA_TABLE).unwrap();
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} photos/vidoes from database.", table.len().unwrap());
        let album_table = txn.open_table(ALBUM_TABLE).unwrap();
        info!(duration = &*format!("{:?}", start_time.elapsed()); "Read {} albums from database", album_table.len().unwrap());
    }

    txn.commit().unwrap();

    SHOULD_RESET.notify_one();

    rocket::build()
        .attach(cache_control_fairing())
        .attach(auth_request_fairing())
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
        .mount("/object/imported", FileServer::from("./object/imported"))
        .mount(
            "/assets",
            FileServer::from("../gallery-frontend/dist/assets"),
        )
        .mount(
            "/",
            routes![
                redirect_to_photo,
                redirect_to_photo_2,
                favicon,
                login,
                compressed_file,
                edit_tag,
                tags,
                favorite,
                favorite_view,
                archived,
                archived_view,
                all,
                all_view,
                setting,
                upload,
                get_data,
                get_config,
                get_tags,
                catch_view_routes,
                unauthorized,
                prefetch,
                generate_random_data,
                get_rows,
                delete_data,
                trashed,
                trashed_view,
                get_scroll_bar,
                regenerate_preview,
                authenticate,
                redirect_to_login,
                create_album,
                authenticate_share,
                get_albums,
                edit_album,
                set_album_cover,
                album_page,
                albums,
                albums_view,
                set_album_title
            ],
        )
}
