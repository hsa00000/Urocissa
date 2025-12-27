use anyhow::Context;
use anyhow::Result;
use std::error::Error;
use std::process::Command;
pub fn video_width_height(info: &str, file_path: &str) -> Result<u32> {
    let command_text = match info {
        "width" => Ok("stream=width"),
        "height" => Ok("stream=height"),
        _ => Err(anyhow::Error::msg("Command error")),
    };
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            command_text?,
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            file_path,
        ])
        .output()
        .context(format!(
            "Fail to spawn new command for ffmpeg: {:?}",
            file_path
        ))?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?.trim().parse::<u32>()?)
    } else {
        Err(anyhow::anyhow!(
            "ffprobe failed for {:?} with status code {:?}: {}",
            file_path,
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn video_duration(file_path: &str) -> Result<f64, Box<dyn Error>> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            file_path,
        ])
        .output()
        .context(format!(
            "Fail to spawn new command for ffmpeg: {:?}",
            file_path
        ))?;
    if output.status.success() {
        let duration_in_seconds = String::from_utf8(output.stdout)?
            .trim()
            .parse::<f64>()
            .context(format!("Fail to parse to f64: {:?}", file_path))?;
        Ok(duration_in_seconds)
    } else {
        Err(From::from(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}
