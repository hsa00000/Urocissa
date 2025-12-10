use rocket::Route;

pub mod execute;

pub fn generate_delete_routes() -> Vec<Route> {
    routes![execute::delete_data]
}
