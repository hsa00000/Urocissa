//! File operations module - handles file system operations
//!
//! Includes:
//! - Random hash generation

use arrayvec::ArrayString;
use rand::{Rng, distr::Alphanumeric};

/// Generate a random 64-character lowercase alphanumeric hash
pub fn generate_random_hash() -> ArrayString<64> {
    let hash: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .take(64)
        .map(char::from)
        .collect();

    ArrayString::<64>::from(&hash).unwrap()
}
