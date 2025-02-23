use std::sync::LazyLock;

use jsonwebtoken::{Algorithm, Validation};
use rocket::Route;

pub mod cache_control_fairing;
pub mod guard_auth;
pub mod guard_read_only_mod;
pub mod guard_share;
pub mod guard_timestamp;

pub fn generate_fairing_routes() -> Vec<Route> {
    routes![guard_timestamp::renew_timestamp_token]
}

static VALIDATION: LazyLock<Validation> = LazyLock::new(|| {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false; // Allow tokens with expired signatures
    validation
});
