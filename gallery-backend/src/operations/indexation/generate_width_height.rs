use super::video_ffprobe::video_width_height;
use crate::public::structure::database_struct::database::definition::Database;
use anyhow::{Context, Result};
use image::DynamicImage;

/// Return `(width, height)` for an already‑decoded **image**.
/// Pure function ‑ no fallible operations.
pub fn generate_image_width_height(dynamic_image: &DynamicImage) -> (u32, u32) {
    (dynamic_image.width(), dynamic_image.height())
}

/// Probe a video file using `ffprobe` (through `video_width_height`) to
/// obtain `(width, height)`, adding explicit context to every `?` site.
pub fn generate_video_width_height(database: &Database) -> Result<(u32, u32)> {
    let imported = database.imported_path_string();

    let width = video_width_height("width", &imported)
        .context(format!("failed to obtain video width for {:?}", imported))?;
    let height = video_width_height("height", &imported)
        .context(format!("failed to obtain video height for {:?}", imported))?;

    Ok((width, height))
}
