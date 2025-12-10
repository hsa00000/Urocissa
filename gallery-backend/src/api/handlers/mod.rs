use rocket::Route;

pub mod album;
pub mod auth;
pub mod execute;
pub mod media;
pub mod share;
pub mod system;

pub fn generate_delete_routes() -> Vec<Route> {
    routes![execute::delete_data]
}
