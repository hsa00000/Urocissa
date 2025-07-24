use std::sync::LazyLock;

use jsonwebtoken::{Algorithm, Validation};
use rocket::Route;

pub mod auth_utils;
pub mod cache_control_fairing;
pub mod guard_auth;
pub mod guard_hash;
pub mod guard_read_only_mode;
pub mod guard_share;
pub mod guard_timestamp;
pub mod guard_upload;

pub fn generate_fairing_routes() -> Vec<Route> {
    routes![
        guard_timestamp::renew_timestamp_token,
        guard_hash::renew_hash_token
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
