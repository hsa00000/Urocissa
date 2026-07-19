use crate::{
    operations::{
        indexation::generate_ffmpeg::create_silent_ffmpeg_command,
        utils::resize::{max_long_side_width_height, small_width_height},
    },
    public::structure::abstract_data::AbstractData,
};
use anyhow::{Context, Result, anyhow};
use image::{DynamicImage, ImageFormat};
use std::{path::Path, process::Stdio};

pub fn generate_thumbnail_for_image_to(
    abstract_data: &AbstractData,
    dynamic_image: &DynamicImage,
    output_path: &Path,
) -> Result<()> {
    let (compressed_width, compressed_height) =
        max_long_side_width_height(abstract_data.width(), abstract_data.height(), 1920);

    let thumbnail_image = dynamic_image
        .thumbnail_exact(compressed_width, compressed_height)
        .to_rgb8();

    // Resolve parent directory of the compressed path
    let parent_path = output_path.parent().ok_or_else(|| {
        anyhow!(
            "failed to determine parent directory of {}",
            output_path.display()
        )
    })?;

    // Ensure the directory exists
    std::fs::create_dir_all(parent_path).context(format!(
        "failed to create directory tree {}",
        parent_path.display()
    ))?;

    // Persist the thumbnail as JPEG
    thumbnail_image
        .save_with_format(output_path, ImageFormat::Jpeg)
        .context(format!(
            "failed to save JPEG thumbnail to {}",
            output_path.display()
        ))?;

    Ok(())
}

pub fn generate_thumbnail_for_video_to(
    abstract_data: &AbstractData,
    output_path: &Path,
) -> Result<()> {
    let (width, height) = (abstract_data.width(), abstract_data.height());
    let (thumb_width, thumb_height) = small_width_height(width, height, 1280);

    // Create target directory tree if missing
    std::fs::create_dir_all(abstract_data.compressed_path_parent())
        .context("failed to create parent directory for video thumbnail")?;

    // Assemble silent ffmpeg command
    let mut cmd = create_silent_ffmpeg_command();
    cmd.args([
        "-y",
        "-i",
        &abstract_data.imported_path_string(),
        "-ss",
        "0",
        "-vframes",
        "1",
        "-vf",
        &format!("scale={thumb_width}:{thumb_height}"),
        &output_path.to_string_lossy(),
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
