use anyhow::{Context, Result, bail};
use image::DynamicImage;
use std::fs::read;
use std::path::Path;

pub fn decode_image(file_path: &Path) -> Result<DynamicImage> {
    let file_in_memory = read(file_path).context(format!(
        "failed to read file into memory: {}",
        file_path.display()
    ))?;

    let decoders = vec![image_crate_decoder];

    for decoder in decoders {
        if let Ok(decoded_image) = decoder(&file_in_memory) {
            return Ok(decoded_image);
        }
    }

    bail!("all decoders failed for file: {}", file_path.display());
}

fn image_crate_decoder(file_in_memory: &[u8]) -> Result<DynamicImage> {
    let dynamic_image = image::load_from_memory(file_in_memory)
        .context("image crate failed to decode image from memory")?;
    Ok(dynamic_image)
}
