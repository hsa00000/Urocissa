use crate::{
    operations::{
        indexation::generate_ffmpeg::create_silent_ffmpeg_command,
        utils::resize::small_width_height,
    },
    public::structure::database_struct::database::definition::Database,
};
use anyhow::{Context, Result, anyhow};
use image::{DynamicImage, ImageFormat};
use std::process::Stdio;

/// Generate a JPEG thumbnail for an **image** asset, propagating
/// every error with clear human‑readable context strings.
pub fn generate_thumbnail_for_image(
    database: &mut Database,
    dynamic_image: DynamicImage,
) -> Result<()> {
    let (compressed_width, compressed_height) =
        small_width_height(database.width, database.height, 1280);

    let thumbnail_image = dynamic_image
        .thumbnail_exact(compressed_width, compressed_height)
        .to_rgb8();

    // Resolve parent directory of the compressed path
    let binding = database.compressed_path();
    let parent_path = binding.parent().ok_or_else(|| {
        anyhow!(
            "failed to determine parent directory of {:?}",
            database.compressed_path()
        )
    })?;

    // Ensure the directory exists
    std::fs::create_dir_all(parent_path)
        .context(format!("failed to create directory tree {:?}", parent_path))?;

    // Persist the thumbnail as JPEG
    thumbnail_image
        .save_with_format(database.compressed_path(), ImageFormat::Jpeg)
        .context(format!(
            "failed to save JPEG thumbnail to {:?}",
            database.compressed_path()
        ))?;

    Ok(())
}

/// Generate a single JPEG thumbnail taken from the **first frame** of a video asset.
/// Uses `ffprobe` for metadata and `ffmpeg` for frame extraction.
/// All fallible operations carry explicit *context* for easier debugging.
pub fn generate_thumbnail_for_video(database: &Database) -> Result<()> {
    let (width, height) = (database.width, database.height);
    let (thumb_width, thumb_height) = small_width_height(width, height, 1280);
    let thumbnail_path = database.thumbnail_path();

    // Create target directory tree if missing
    std::fs::create_dir_all(database.compressed_path_parent())
        .context("failed to create parent directory for video thumbnail")?;

    // Assemble silent ffmpeg command
    let mut cmd = create_silent_ffmpeg_command();
    cmd.args([
        "-y",
        "-i",
        &database.imported_path_string(),
        "-ss",
        "0",
        "-vframes",
        "1",
        "-vf",
        &format!("scale={}:{}", thumb_width, thumb_height),
        &thumbnail_path,
    ]);

    // Execute and wait; we discard both stdout/stderr
    let status = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to execute ffmpeg for video thumbnail generation")?;

    if !status.success() {
        return Err(anyhow!(
            "ffmpeg thumbnail generation failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}
