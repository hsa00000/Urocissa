use crate::operations::indexation::{
    fix_orientation::{fix_image_orientation, fix_image_width_height, fix_video_width_height},
    generate_dynamic_image::generate_dynamic_image,
    generate_exif::{generate_exif_for_image, generate_exif_for_video},
    generate_image_hash::{generate_phash, generate_thumbhash},
    generate_thumbnail::{generate_thumbnail_for_image, generate_thumbnail_for_video},
    generate_width_height::{generate_image_width_height, generate_video_width_height},
};
use crate::public::structure::database::definition::DatabaseSchema;
use anyhow::{Context, Result};

/// Analyse the newly‑imported **image** and populate the `DatabaseSchema` record.
pub fn process_image_info(database: &mut DatabaseSchema) -> Result<()> {
    // EXIF metadata extraction (non‑fallible)
    database.exif_vec = generate_exif_for_image(database);

    // Decode image to DynamicImage
    let mut dynamic_image =
        generate_dynamic_image(database).context("failed to decode image into DynamicImage")?;

    // Measure & possibly fix width/height
    (database.width, database.height) = generate_image_width_height(&dynamic_image);
    fix_image_width_height(database);

    // Adjust orientation if required
    fix_image_orientation(database, &mut dynamic_image);

    // Compute perceptual hashes
    database.thumbhash = generate_thumbhash(&dynamic_image);
    database.phash = generate_phash(&dynamic_image);

    // Generate on‑disk JPEG thumbnail
    generate_thumbnail_for_image(database, dynamic_image)
        .context("failed to generate JPEG thumbnail for image")?;

    // Compute timestamp_ms from EXIF if possible
    if let Some(ts) = database.compute_timestamp_ms_by_exif() {
        database.timestamp_ms = ts;
    }

    Ok(())
}

/// Re‑build all metadata for an existing **image** (e.g. after replace / fix).
pub fn regenerate_metadata_for_image(database: &mut DatabaseSchema) -> Result<()> {
    // Refresh size from filesystem
    database.size = std::fs::metadata(database.imported_path())
        .context("failed to read metadata for imported image file")?
        .len();

    // Re‑run the full processing pipeline
    process_image_info(database).context("failed to process image info")?;
    Ok(())
}

/// Analyse the newly‑imported **video** and populate the `DatabaseSchema` record.
pub fn process_video_info(database: &mut DatabaseSchema) -> Result<()> {
    // Extract EXIF‑like metadata via ffprobe
    database.exif_vec = generate_exif_for_video(database)
        .context("failed to extract video metadata via ffprobe")?;

    // Get logical dimensions and fix if rotated
    (database.width, database.height) =
        generate_video_width_height(database).context("failed to obtain video width/height")?;
    fix_video_width_height(database);

    // Produce thumbnail from first frame
    generate_thumbnail_for_video(database)
        .context("failed to generate video thumbnail via ffmpeg")?;

    // Decode the first frame for hashing purposes
    let dynamic_image = generate_dynamic_image(database)
        .context("failed to decode first video frame into DynamicImage")?;

    // Compute perceptual hashes
    database.thumbhash = generate_thumbhash(&dynamic_image);
    database.phash = generate_phash(&dynamic_image);

    Ok(())
}

/// Re‑build all metadata for an existing **video** file.
pub fn regenerate_metadata_for_video(database: &mut DatabaseSchema) -> Result<()> {
    // Refresh size from filesystem metadata
    database.size = std::fs::metadata(database.imported_path())
        .context("failed to read metadata for imported video file")?
        .len();

    // Re‑run the full processing pipeline
    process_video_info(database).context("failed to process video info")?;
    Ok(())
}
