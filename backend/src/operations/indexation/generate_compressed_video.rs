use super::video_ffprobe::video_duration;
use crate::{
    operations::indexation::generate_ffmpeg::create_silent_ffmpeg_command,
    process::info::process_image_info,
    public::{structure::abstract_data::AbstractData, tui::DASHBOARD},
};
use anyhow::Context;
use anyhow::Result;
use log::info;
use regex::Regex;
use std::{
    cmp,
    io::{BufRead, BufReader},
    process::Stdio,
    sync::LazyLock,
};

static REGEX_OUT_TIME_US: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"out_time_us=(\d+)").unwrap());

/// Compresses a video file, reporting progress by parsing ffmpeg's output.
pub fn generate_compressed_video(abstract_data: &mut AbstractData) -> Result<()> {
    let duration_result = video_duration(&abstract_data.imported_path_string());
    let duration = match duration_result {
        // Handle static GIFs by delegating to the image processor.
        Ok(d) if (d * 1000.0) as u32 == 100 => {
            info!(
                "Static GIF detected. Processing as image: {:?}",
                abstract_data.imported_path_string()
            );
            abstract_data.convert_to_image();
            return process_image_info(abstract_data);
        }
        // Handle non-GIFs that fail to parse duration.
        Err(err)
            if err.to_string().contains("fail to parse to f32")
                && abstract_data.ext().eq_ignore_ascii_case("gif") =>
        {
            info!(
                "Potentially corrupt or non-standard GIF. Processing as image: {:?}",
                abstract_data.imported_path_string()
            );
            abstract_data.convert_to_image();
            return process_image_info(abstract_data);
        }
        Ok(d) => d,
        Err(err) => {
            return Err(anyhow::anyhow!(
                "Failed to get video duration for {:?}: {}",
                abstract_data.imported_path_string(),
                err
            ));
        }
    };
    // --- REFACTORED: Use the helper for a clean, consistent command ---
    let mut cmd = create_silent_ffmpeg_command();
    cmd.args([
        "-y", // Overwrite output file if it exists
        "-i",
        &abstract_data.imported_path_string(),
        "-vf",
        // Scale video to a max height of 720p, ensuring dimensions are even.
        &format!(
            "scale=trunc(oh*a/2)*2:{}",
            (cmp::min(abstract_data.height(), 720) / 2) * 2
        ),
        "-movflags",
        "faststart", // Optimize for web streaming
        &abstract_data.compressed_path_string(),
        "-progress",
        "pipe:2", // Send machine-readable progress to stderr (pipe 2)
    ]);

    // We capture stderr for progress parsing and discard stdout completely.
    let mut child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn ffmpeg for video compression")?;

    let stderr = child
        .stderr
        .take()
        .context("Failed to capture ffmpeg stderr")?;
    let reader = BufReader::new(stderr);

    // Process each line of progress output from ffmpeg's stderr.
    for line_result in reader.lines() {
        if let Ok(line) = line_result {
            if let Some(caps) = REGEX_OUT_TIME_US.captures(&line) {
                // The regex now captures either digits or "N/A".
                // We only proceed if the captured value can be parsed as a number.
                if let Ok(microseconds) = caps[1].parse::<f64>() {
                    let percentage = (microseconds / 1_000_000.0 / duration) * 100.0;
                    DASHBOARD.update_progress(abstract_data.hash(), percentage);
                }
            }
        }
    }

    child
        .wait()
        .context("Failed to wait for ffmpeg child process")?;
    Ok(())
}
