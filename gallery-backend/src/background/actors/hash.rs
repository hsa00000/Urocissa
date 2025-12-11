use crate::common::WORKER_RAYON_POOL;
use crate::common::errors::handle_error;
use anyhow::{Context, Result};
use arrayvec::ArrayString;
use blake3::Hasher;
use mini_executor::Task;
use std::{fs::File, io::Read};
use tokio_rayon::AsyncThreadPool;

pub struct HashTask {
    pub file: File,
}

impl HashTask {
    pub fn new(file: File) -> Self {
        Self { file }
    }
}

impl Task for HashTask {
    type Output = Result<ArrayString<64>>;

    fn run(self) -> impl Future<Output = Self::Output> + Send {
        async move {
            WORKER_RAYON_POOL
                .spawn_async(move || hash_task(self.file))
                .await
                .map_err(|err| handle_error(err.context("Failed to run hash task")))
        }
    }
}
fn hash_task(file: File) -> Result<ArrayString<64>> {
    blake3_hasher(file)
}

/// Compute Blake3 hash of a file
pub fn blake3_hasher(mut file: File) -> Result<ArrayString<64>> {
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 512 * 1024];

    loop {
        let n = file.read(&mut buffer).context("Failed to read file")?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().to_hex())
}
