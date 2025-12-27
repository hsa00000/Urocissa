use anyhow::{Context, Result};
use arrayvec::ArrayString;
use blake3::Hasher;
use rand::{Rng, distr::Alphanumeric};
use std::{fs::File, io::Read};

pub fn blake3_hasher(mut file: File) -> Result<ArrayString<64>> {
    let mut hasher = Hasher::new(); // :contentReference[oaicite:5]{index=5}
    let mut buffer = [0u8; 512 * 1024];

    loop {
        let n = file
            .read(&mut buffer)
            .context(format!("Failed to read file"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().to_hex())
}

pub fn generate_random_hash() -> ArrayString<64> {
    let hash: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .take(64)
        .map(char::from)
        .collect();

    let hash = ArrayString::<64>::from(&hash).unwrap();
    hash
}
