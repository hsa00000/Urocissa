use crate::public::structure::database_struct::database::definition::Database;
use anyhow::{Context, Result, bail};
use image::DynamicImage;
use std::fs::read;
use std::path::PathBuf;

/// Generate a `DynamicImage` either from the original image or
/// from its thumbnail, adding *context* at every fallible step.
pub fn generate_dynamic_image(database: &Database) -> Result<DynamicImage> {
    let img_path = if database.ext_type == "image" {
        database.imported_path()
    } else {
        PathBuf::from(database.thumbnail_path())
    };

    let dynamic_image =
        decode_image(&img_path).context(format!("failed to decode image: {:?}", img_path))?;

    Ok(dynamic_image)
}

fn decode_image(file_path: &PathBuf) -> Result<DynamicImage> {
    let file_in_memory =
        read(file_path).context(format!("failed to read file into memory: {:?}", file_path))?;

    let decoders: Vec<fn(&Vec<u8>) -> Result<DynamicImage>> = vec![image_crate_decoder];

    for decoder in decoders {
        match decoder(&file_in_memory) {
            Ok(decoded_image) => return Ok(decoded_image),
            Err(_) => continue,
        }
    }

    bail!("all decoders failed for file: {:?}", file_path);
}

fn image_crate_decoder(file_in_memory: &Vec<u8>) -> Result<DynamicImage> {
    let dynamic_image = image::load_from_memory(file_in_memory)
        .context("image crate failed to decode image from memory")?;
    Ok(dynamic_image)
}
