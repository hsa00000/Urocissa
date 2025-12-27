use std::fs::metadata;

use crate::operations::indexation::fix_orientation::{
    fix_image_orientation, fix_image_width_height, fix_video_width_height,
};
use crate::operations::indexation::generate_dynamic_image::generate_dynamic_image;
use crate::operations::indexation::generate_exif::{
    generate_exif_for_image, generate_exif_for_video,
};
use crate::operations::indexation::generate_image_hash::{generate_phash, generate_thumbhash};
use crate::operations::indexation::generate_thumbnail::{
    generate_thumbnail_for_image, generate_thumbnail_for_video,
};
use crate::operations::indexation::generate_width_height::{
    generate_image_width_height, generate_video_width_height,
};
use crate::public::structure::abstract_data::AbstractData;
use anyhow::{Context, Result};

/// Analyse the newly‑imported **image** and populate the `AbstractData` record.
pub fn process_image_info(abstract_data: &mut AbstractData) -> Result<()> {
    // EXIF metadata extraction (non‑fallible)
    // Generate exif first, then assign it to avoid borrow issues
    let exif_data = generate_exif_for_image(abstract_data);
    if let Some(exif_vec) = abstract_data.exif_vec_mut() {
        *exif_vec = exif_data;
    }

    // Decode image to DynamicImage
    let mut dynamic_image =
        generate_dynamic_image(abstract_data).context("failed to decode image into DynamicImage")?;

    // Measure & possibly fix width/height
    let (width, height) = generate_image_width_height(&dynamic_image);
    abstract_data.set_width(width);
    abstract_data.set_height(height);
    fix_image_width_height(abstract_data);

    // Adjust orientation if required
    fix_image_orientation(abstract_data, &mut dynamic_image);

    // Compute perceptual hashes
    abstract_data.set_thumbhash(generate_thumbhash(&dynamic_image));
    abstract_data.set_phash(generate_phash(&dynamic_image));

    // Generate on‑disk JPEG thumbnail
    generate_thumbnail_for_image(abstract_data, dynamic_image)
        .context("failed to generate JPEG thumbnail for image")?;

    Ok(())
}

/// Re‑build all metadata for an existing **image** (e.g. after replace / fix).
pub fn regenerate_metadata_for_image(abstract_data: &mut AbstractData) -> Result<()> {
    // Refresh size from filesystem
    let size = metadata(abstract_data.imported_path())
        .context("failed to read metadata for imported image file")?
        .len();
    abstract_data.set_size(size);

    // Re‑run the full processing pipeline
    process_image_info(abstract_data).context("failed to process image info")?;
    Ok(())
}

/// Analyse the newly‑imported **video** and populate the `AbstractData` record.
pub fn process_video_info(abstract_data: &mut AbstractData) -> Result<()> {
    // Extract EXIF‑like metadata via ffprobe
    let exif = generate_exif_for_video(abstract_data)
        .context("failed to extract video metadata via ffprobe")?;
    if let Some(exif_vec) = abstract_data.exif_vec_mut() {
        *exif_vec = exif;
    }

    // Get logical dimensions and fix if rotated
    let (width, height) =
        generate_video_width_height(abstract_data).context("failed to obtain video width/height")?;
    abstract_data.set_width(width);
    abstract_data.set_height(height);
    fix_video_width_height(abstract_data);

    // Produce thumbnail from first frame
    generate_thumbnail_for_video(abstract_data)
        .context("failed to generate video thumbnail via ffmpeg")?;

    // Decode the first frame for hashing purposes
    let dynamic_image = generate_dynamic_image(abstract_data)
        .context("failed to decode first video frame into DynamicImage")?;

    // Compute perceptual hashes
    abstract_data.set_thumbhash(generate_thumbhash(&dynamic_image));
    abstract_data.set_phash(generate_phash(&dynamic_image));

    Ok(())
}

/// Re‑build all metadata for an existing **video** file.
pub fn regenerate_metadata_for_video(abstract_data: &mut AbstractData) -> Result<()> {
    // Refresh size from filesystem metadata
    let size = metadata(abstract_data.imported_path())
        .context("failed to read metadata for imported video file")?
        .len();
    abstract_data.set_size(size);

    // Re‑run the full processing pipeline
    process_video_info(abstract_data).context("failed to process video info")?;
    Ok(())
}
