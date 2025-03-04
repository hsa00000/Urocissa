use std::sync::LazyLock;

use jsonwebtoken::{Algorithm, Validation};
use rocket::Route;

pub mod auth_guard;
pub mod cache_control_fairing;
pub mod hash_guard;
pub mod read_only_mod_guard;
pub mod share_guard;
pub mod timestamp_guard;

pub fn generate_fairing_routes() -> Vec<Route> {
    routes![
        timestamp_guard::renew_timestamp_token,
        hash_guard::renew_hash_token
    ]
}

static VALIDATION: LazyLock<Validation> = LazyLock::new(|| {
    let validation = Validation::new(Algorithm::HS256);
    validation
});

static VALIDATION_ALLOW_EXPIRED: LazyLock<Validation> = LazyLock::new(|| {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // Disable expiration validation
    validation
});
