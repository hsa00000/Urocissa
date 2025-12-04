//! Image processing module - handles all static image related logic
//!
//! Includes:
//! - Image decoding
//! - Orientation correction
//! - Width/height calculation
//! - Thumbnail generation
//! - Perceptual hash (thumbhash, phash) computation

use crate::{
    public::structure::abstract_data::{Database, MediaWithAlbum},
    public::structure::database::generate_timestamp::compute_timestamp_ms_by_exif,
    workflow::tasks::actor::index::IndexTask,
};
use anyhow::{Context, Result, anyhow, bail};
use image::{DynamicImage, ImageFormat};
use image_hasher::HasherConfig;
use std::{collections::BTreeMap, fs::read, path::PathBuf};

use super::metadata::generate_exif_for_image;

// ────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────

/// Analyse the newly-imported image and populate the DatabaseSchema record
pub fn process_image_info(index_task: &mut IndexTask) -> Result<()> {
    // EXIF metadata extraction (non-fallible)
    generate_exif_for_image(index_task);

    // Decode image to DynamicImage
    let mut dynamic_image =
        generate_dynamic_image(index_task).context("failed to decode image into DynamicImage")?;

    // Measure and possibly fix width/height
    (index_task.width, index_task.height) = generate_image_width_height(&dynamic_image);
    fix_image_width_height(index_task);

    // Adjust orientation if required
    fix_image_orientation(&index_task.exif_vec, &mut dynamic_image);

    // Compute perceptual hashes
    index_task.thumbhash = generate_thumbhash(&dynamic_image);
    index_task.phash = generate_phash(&dynamic_image);

    // Generate on-disk JPEG thumbnail
    generate_thumbnail_for_image(index_task, dynamic_image)
        .context("failed to generate JPEG thumbnail for image")?;

    // Compute timestamp_ms from EXIF if possible
    compute_timestamp_ms_by_exif(index_task);

    Ok(())
}

/// Rebuild all metadata for an existing image (e.g. after replace/fix)
/// [FIX] Now returns the EXIF data collected during processing
pub fn regenerate_metadata_for_image(database: &mut Database) -> Result<BTreeMap<String, String>> {
    // Refresh size from filesystem - we need to update the underlying ImageCombined
    let new_size = std::fs::metadata(database.imported_path())
        .context("failed to read metadata for imported image file")?
        .len();

    // Update the size in the ImageCombined
    if let MediaWithAlbum::Image(ref mut img) = database.media {
        img.metadata.size = new_size;
    }

    // Re-run the full processing pipeline
    let mut index_task = IndexTask::new(database.imported_path(), database.media.clone());
    process_image_info(&mut index_task).context("failed to process image info")?;
    let exif_vec = index_task.exif_vec.clone();
    database.media = index_task.into();

    // [FIX] Return the EXIF data
    Ok(exif_vec)
} // ────────────────────────────────────────────────────────────────
// DynamicImage Generation
// ────────────────────────────────────────────────────────────────

/// Generate a `DynamicImage` from either the original image or its thumbnail
pub fn generate_dynamic_image(index_task: &IndexTask) -> Result<DynamicImage> {
    let img_path = if index_task.ext_type == "image" {
        index_task.imported_path()
    } else {
        PathBuf::from(index_task.thumbnail_path())
    };

    let dynamic_image = generate_dynamic_image_from_path(&img_path)
        .context(format!("failed to decode image: {:?}", img_path))?;

    Ok(dynamic_image)
}

pub fn generate_dynamic_image_from_path(file_path: &PathBuf) -> Result<DynamicImage> {
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

// ────────────────────────────────────────────────────────────────
// Orientation Correction
// ────────────────────────────────────────────────────────────────

pub fn fix_image_orientation(
    exif_vec: &BTreeMap<String, String>,
    dynamic_image: &mut DynamicImage,
) {
    if let Some(orientation) = exif_vec.get("Orientation") {
        match orientation.as_str() {
            "row 0 at right and column 0 at top" => {
                *dynamic_image = dynamic_image.rotate90();
            }
            "row 0 at bottom and column 0 at right" => {
                *dynamic_image = dynamic_image.rotate180();
            }
            "row 0 at left and column 0 at bottom" => {
                *dynamic_image = dynamic_image.rotate270();
            }
            _ => (),
        }
    }
}

pub fn fix_image_width_height(index_task: &mut IndexTask) {
    if let Some(orientation) = index_task.exif_vec.get("Orientation") {
        match orientation.as_str() {
            "row 0 at right and column 0 at top" => {
                std::mem::swap(&mut index_task.width, &mut index_task.height)
            }
            "row 0 at left and column 0 at bottom" => {
                std::mem::swap(&mut index_task.width, &mut index_task.height)
            }
            _ => (),
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Width/Height Calculation
// ────────────────────────────────────────────────────────────────

/// Return `(width, height)` for an already-decoded image
pub fn generate_image_width_height(dynamic_image: &DynamicImage) -> (u32, u32) {
    (dynamic_image.width(), dynamic_image.height())
}

// ────────────────────────────────────────────────────────────────
// Thumbnail Generation
// ────────────────────────────────────────────────────────────────

/// Generate a JPEG thumbnail for an image asset
pub fn generate_thumbnail_for_image(
    index_task: &mut IndexTask,
    dynamic_image: DynamicImage,
) -> Result<()> {
    let (compressed_width, compressed_height) =
        small_width_height(index_task.width, index_task.height, 1280);
    let thumbnail_image = dynamic_image
        .thumbnail_exact(compressed_width, compressed_height)
        .to_rgb8();

    // Resolve parent directory of the compressed path
    let binding = index_task.compressed_path();
    let parent_path = binding.parent().ok_or_else(|| {
        anyhow!(
            "failed to determine parent directory of {:?}",
            index_task.compressed_path()
        )
    })?;

    // Ensure the directory exists
    std::fs::create_dir_all(parent_path)
        .context(format!("failed to create directory tree {:?}", parent_path))?;

    // Persist the thumbnail as JPEG
    thumbnail_image
        .save_with_format(index_task.compressed_path(), ImageFormat::Jpeg)
        .context(format!(
            "failed to save JPEG thumbnail to {:?}",
            index_task.compressed_path()
        ))?;

    Ok(())
}

// ────────────────────────────────────────────────────────────────
// Perceptual Hash
// ────────────────────────────────────────────────────────────────

pub fn generate_thumbhash(dynamic_image_rotated: &DynamicImage) -> Vec<u8> {
    let resized_image = dynamic_image_rotated.thumbnail_exact(100, 100);
    let rgba_image = resized_image.to_rgba8();
    let (swidth, sheight) = (rgba_image.width(), rgba_image.height());
    thumbhash::rgba_to_thumb_hash(swidth as usize, sheight as usize, &rgba_image)
}

pub fn generate_phash(dynamic_image_rotated: &DynamicImage) -> Vec<u8> {
    let hasher = HasherConfig::new().to_hasher();
    let phash = hasher.hash_image(dynamic_image_rotated);
    phash.as_bytes().to_vec()
}

// ────────────────────────────────────────────────────────────────
// Helper Functions
// ────────────────────────────────────────────────────────────────

/// Resize dimensions so that the larger side equals `small_height`, preserving aspect ratio
pub fn small_width_height(width: u32, height: u32, small_height: u32) -> (u32, u32) {
    let (nwidth, nheight) = if width >= std::cmp::max(height, small_height) {
        (small_height, height * small_height / width)
    } else if height >= std::cmp::max(width, small_height) {
        (width * small_height / height, small_height)
    } else {
        (width, height)
    };

    (nwidth, nheight)
}
