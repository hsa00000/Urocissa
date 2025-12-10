pub mod cache;
pub mod guards;
pub mod utils;

use rocket::Route;
use std::sync::LazyLock;
use jsonwebtoken::{Validation, Algorithm};

pub fn generate_fairing_routes() -> Vec<Route> {
    routes![
        guards::timestamp::renew_timestamp_token,
        guards::hash::renew_hash_token
    ]
}

pub static VALIDATION: LazyLock<Validation> = LazyLock::new(|| {
    let validation = Validation::new(Algorithm::HS256);
    validation
});

pub static VALIDATION_ALLOW_EXPIRED: LazyLock<Validation> = LazyLock::new(|| {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // Disable expiration validation
    validation
});
