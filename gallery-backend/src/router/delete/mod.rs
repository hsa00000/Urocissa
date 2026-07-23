// src/router/delete/mod.rs
use rocket::Route;

pub mod delete_data;

pub fn generate_delete_routes() -> Vec<Route> {
    routes![
        delete_data::delete_data,
        crate::router::saved_searches::delete_saved_search
    ]
}
