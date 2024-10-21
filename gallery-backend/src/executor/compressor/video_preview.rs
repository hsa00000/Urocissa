use super::{image_thumbhash::generate_thumbhash, utils::small_width_height};
use crate::public::database_struct::database::definition::DataBase;
use anyhow::Context;
use std::{error::Error, path::PathBuf, process::Command};

pub fn generate_preview(database: &mut DataBase) -> Result<(), Box<dyn Error>> {
    let width = database.width;
    let height = database.height;

    let (preview_width, preview_height) = small_width_height(width, height, 1280);

    let preview_scale_args = format!("scale={}:{}", preview_width, preview_height);

    let preview_file_path_string = &database.preview_path();

    let status = Command::new("ffmpeg")
        .args(&[
            "-y",
            "-i",
            &database.imported_path_string(),
            "-ss",
            "0",
            "-frames:v",
            "1", // Generate only one image
            "-vf",
            &preview_scale_args,
            preview_file_path_string,
        ])
        .status()
        .with_context(|| {
            format!(
                "generate_preview: failed to spawn new command for ffmpeg: {:?}",
                preview_file_path_string
            )
        })?;

    if !status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "ffmpeg failed to generate preview",
        )));
    }
    generate_thumbhash(database, &PathBuf::from(database.preview_path()))?;
    Ok(())
}