use crate::{
    workflow::operations::indexation::{
        fix_orientation::{fix_image_orientation, fix_image_width_height, fix_video_width_height},
        generate_dynamic_image::generate_dynamic_image,
        generate_exif::{generate_exif_for_image, generate_exif_for_video},
        generate_image_hash::{generate_phash, generate_thumbhash},
        generate_thumbnail::{generate_thumbnail_for_image, generate_thumbnail_for_video},
        generate_width_height::{generate_image_width_height, generate_video_width_height},
    },
    public::structure::database::generate_timestamp::compute_timestamp_ms_by_exif,
    table::database::DatabaseSchema,
    workflow::tasks::actor::index::IndexTask,
};
use anyhow::{Context, Result};

/// Analyse the newly‑imported **image** and populate the `DatabaseSchema` record.
pub fn process_image_info(index_task: &mut IndexTask) -> Result<()> {
    // EXIF metadata extraction (non‑fallible)
    generate_exif_for_image(index_task);

    // Decode image to DynamicImage
    let mut dynamic_image =
        generate_dynamic_image(index_task).context("failed to decode image into DynamicImage")?;

    // Measure & possibly fix width/height
    (index_task.width, index_task.height) = generate_image_width_height(&dynamic_image);
    fix_image_width_height(index_task);

    // Adjust orientation if required
    fix_image_orientation(&index_task.exif_vec, &mut dynamic_image);

    // Compute perceptual hashes
    index_task.thumbhash = generate_thumbhash(&dynamic_image);
    index_task.phash = generate_phash(&dynamic_image);
    // Generate on‑disk JPEG thumbnail
    generate_thumbnail_for_image(index_task, dynamic_image)
        .context("failed to generate JPEG thumbnail for image")?;

    // Compute timestamp_ms from EXIF if possible
    compute_timestamp_ms_by_exif(index_task);

    Ok(())
}

/// Re‑build all metadata for an existing **image** (e.g. after replace / fix).
pub fn regenerate_metadata_for_image(database: &mut DatabaseSchema) -> Result<()> {
    // Refresh size from filesystem
    database.size = std::fs::metadata(database.imported_path())
        .context("failed to read metadata for imported image file")?
        .len();

    // Re‑run the full processing pipeline
    let mut index_task = IndexTask::new(database.imported_path(), database.clone());
    process_image_info(&mut index_task).context("failed to process image info")?;
    *database = index_task.into();

    Ok(())
}

/// Analyse the newly‑imported **video** and populate the `DatabaseSchema` record.
pub fn process_video_info(index_task: &mut IndexTask) -> Result<()> {
    // Extract EXIF‑like metadata via ffprobe
    generate_exif_for_video(index_task).context("failed to extract video metadata via ffprobe")?;

    // Get logical dimensions and fix if rotated
    (index_task.width, index_task.height) =
        generate_video_width_height(&index_task).context("failed to obtain video width/height")?;
    fix_video_width_height(index_task);

    // Produce thumbnail from first frame
    generate_thumbnail_for_video(index_task)
        .context("failed to generate video thumbnail via ffmpeg")?;

    // Decode the first frame for hashing purposes
    let dynamic_image = generate_dynamic_image(&index_task)
        .context("failed to decode first video frame into DynamicImage")?;

    // Compute perceptual hashes
    index_task.thumbhash = generate_thumbhash(&dynamic_image);
    index_task.phash = generate_phash(&dynamic_image);

    Ok(())
}

/// Re‑build all metadata for an existing **video** file.
pub fn regenerate_metadata_for_video(index_task: &mut IndexTask) -> Result<()> {
    // Refresh size from filesystem metadata
    index_task.size = std::fs::metadata(index_task.imported_path())
        .context("failed to read metadata for imported video file")?
        .len();

    // Re‑run the full processing pipeline
    process_video_info(index_task).context("failed to process video info")?;
    Ok(())
}
