use anyhow::Result;
use log::warn;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::public::constant::MAX_COPY_RETRIES;

pub fn copy_with_retry(src: &Path, dst: &Path) -> Result<u64> {
    for attempt in 1..=MAX_COPY_RETRIES + 1 {
        match fs::copy(src, dst) {
            Ok(bytes) => return Ok(bytes),
            Err(error) if attempt <= MAX_COPY_RETRIES => {
                warn!(
                    "File copy failed (attempt {}/{}): {:?} → {:?}\nError: {}\nRetrying in 1 second...",
                    attempt,
                    MAX_COPY_RETRIES + 1,
                    src,
                    dst,
                    error
                );
                thread::sleep(Duration::from_secs(1));
            }
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!("loop guarantees return")
}
