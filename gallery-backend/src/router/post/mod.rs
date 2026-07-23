// src/router/post/mod.rs
use rocket::Route;
pub mod authenticate;
pub mod create_album;
pub mod create_share;
pub mod import_config;
pub mod import_folder;
pub mod post_upload;

pub fn generate_post_routes() -> Vec<Route> {
    routes![
        authenticate::authenticate,
        create_album::create_non_empty_album,
        create_album::create_empty_album,
        post_upload::upload,
        create_share::create_share,
        import_config::import_config_handler,
        import_folder::start_folder_import_handler,
        import_folder::cancel_folder_import_handler,
        crate::router::saved_searches::create_saved_search
    ]
}
