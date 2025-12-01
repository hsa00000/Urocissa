//! File operations module - handles file system operations
//!
//! Includes:
//! - Blake3 hash computation
//! - Random hash generation
//! - File opening with retry logic

use anyhow::{Context, Error, Result};
use arrayvec::ArrayString;
use blake3::Hasher;
use log::warn;
use rand::{Rng, distr::Alphanumeric};
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    thread::sleep,
    time::Duration,
};

const OPEN_FAIL_RETRY: usize = 3;
const OPEN_RETRY_DELAY_MS: u64 = 100;

// ────────────────────────────────────────────────────────────────
// Hash Computation
// ────────────────────────────────────────────────────────────────

/// Compute Blake3 hash of a file
pub fn blake3_hasher(mut file: File) -> Result<ArrayString<64>> {
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 512 * 1024];

    loop {
        let n = file
            .read(&mut buffer)
            .context("Failed to read file")?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().to_hex())
}

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

// ────────────────────────────────────────────────────────────────
// File Opening
// ────────────────────────────────────────────────────────────────

/// Open a file with retry logic for transient failures
pub fn open_file_with_retry(path: PathBuf) -> Result<File> {
    let mut delay = Duration::from_millis(OPEN_RETRY_DELAY_MS);

    for attempt in 0..=OPEN_FAIL_RETRY {
        match File::open(&path) {
            Ok(file) => return Ok(file),
            Err(e) if attempt < OPEN_FAIL_RETRY => {
                warn!(
                    "Attempt {}/{} failed to open {:?}: {}. Retrying in {:?}…",
                    attempt + 1,
                    OPEN_FAIL_RETRY + 1,
                    path,
                    e,
                    delay,
                );
                sleep(delay);
                delay = delay.checked_mul(2).unwrap_or(delay);
            }
            Err(e) => {
                return Err(Error::new(e).context(format!(
                    "Failed to open file {:?} after {} attempts",
                    path,
                    OPEN_FAIL_RETRY + 1
                )));
            }
        }
    }

    unreachable!("open_file_with_retry logic error")
}
