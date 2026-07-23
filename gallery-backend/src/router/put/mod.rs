// src/router/put/mod.rs
use rocket::Route;

pub mod edit_album;
pub mod edit_config;
pub mod edit_description;
pub mod edit_flags;
pub mod edit_share;
pub mod edit_tag;
pub mod regenerate_thumbnail;
pub mod reindex;
pub mod rotate_image;
pub fn generate_put_routes() -> Vec<Route> {
    routes![
        edit_album::edit_album,
        edit_album::set_album_cover,
        edit_album::set_album_title,
        edit_description::set_user_defined_description,
        edit_flags::edit_flags,
        edit_share::edit_share,
        edit_share::delete_share,
        edit_tag::edit_tag,
        regenerate_thumbnail::regenerate_thumbnail_with_frame,
        reindex::reindex,
        reindex::cancel_reindex,
        edit_config::update_config_handler,
        edit_config::update_password_handler,
        rotate_image::rotate_image,
        crate::router::saved_searches::rename_saved_search,
        crate::router::saved_searches::reorder_saved_searches
    ]
}
