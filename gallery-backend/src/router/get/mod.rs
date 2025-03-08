use rocket::Route;

pub mod get_data;
pub mod get_export;
pub mod get_img;
pub mod get_page;
pub mod get_prefetch;
pub mod get_timestamp;

pub fn generate_get_routes() -> Vec<Route> {
    routes![
        get_data::get_config,
        get_data::get_tags,
        get_data::get_albums,
        get_timestamp::get_data,
        get_timestamp::get_rows,
        get_timestamp::get_scroll_bar,
        get_img::compressed_file,
        get_img::imported_file,
        get_page::redirect_to_photo,
        get_page::login,
        get_page::redirect_to_login,
        get_page::unauthorized,
        get_page::home,
        get_page::home_view,
        get_page::tags,
        get_page::favorite,
        get_page::favorite_view,
        get_page::albums,
        get_page::albums_view,
        get_page::album_page,
        get_page::share_view,
        get_page::archived,
        get_page::archived_view,
        get_page::trashed,
        get_page::trashed_view,
        get_page::all,
        get_page::all_view,
        get_page::setting,
        get_page::favicon,
        get_page::videos,
        get_page::videos_view,
        get_page::service_worker,
        get_page::sregister_sw,
        get_prefetch::prefetch,
        get_export::get_export
    ]
}
