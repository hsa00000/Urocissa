use super::video_ffprobe::video_duration;
use crate::{
    operations::indexation::generate_ffmpeg::create_silent_ffmpeg_command,
    public::{structure::abstract_data::AbstractData, tui::DASHBOARD},
};
use anyhow::Context;
use anyhow::Result;
use log::info;
use regex::Regex;
use std::{
    cmp,
    io::{BufRead, BufReader},
    path::Path,
    process::Stdio,
    sync::LazyLock,
};

static REGEX_OUT_TIME_US: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"out_time_us=(\d+)").unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCompressionOutcome {
    Video,
    StaticImage,
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn is_static_gif(abstract_data: &AbstractData) -> Result<bool> {
    match video_duration(&abstract_data.imported_path_string()) {
        Ok(duration) => Ok((duration * 1000.0) as u32 == 100),
        Err(error)
            if (error.to_string().contains("fail to parse to f32")
                || error.to_string().contains("Fail to parse to f64"))
                && abstract_data.ext().eq_ignore_ascii_case("gif") =>
        {
            Ok(true)
        }
        Err(error) => Err(anyhow::anyhow!(
            "Failed to get video duration for {:?}: {}",
            abstract_data.imported_path_string(),
            error
        )),
    }
}

/// Compress into a caller-provided job-scoped path. Static GIF conversion is
/// reported to the shared media pipeline so it can stage the image artifacts
/// and publish the type change atomically.
pub fn generate_compressed_video_to(
    abstract_data: &AbstractData,
    output_path: &Path,
    report_progress: bool,
) -> Result<VideoCompressionOutcome> {
    if is_static_gif(abstract_data)? {
        info!(
            "Static GIF detected. Processing as image: {:?}",
            abstract_data.imported_path_string()
        );
        return Ok(VideoCompressionOutcome::StaticImage);
    }
    let duration_result = video_duration(&abstract_data.imported_path_string());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let duration = match duration_result {
        Ok(d) => d,
        // Handle non-GIFs that fail to parse duration.
        Err(err)
            if err.to_string().contains("fail to parse to f32")
                && abstract_data.ext().eq_ignore_ascii_case("gif") =>
        {
            info!(
                "Potentially corrupt or non-standard GIF. Processing as image: {:?}",
                abstract_data.imported_path_string()
            );
            return Ok(VideoCompressionOutcome::StaticImage);
        }
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
            (cmp::min(abstract_data.height(), 720).max(2) / 2) * 2
        ),
        "-movflags",
        "faststart", // Optimize for web streaming
        &output_path.to_string_lossy(),
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
    for line in reader.lines().map_while(Result::ok) {
        if let Some(caps) = REGEX_OUT_TIME_US.captures(&line) {
            // The regex now captures either digits or "N/A".
            // We only proceed if the captured value can be parsed as a number.
            if let Ok(microseconds) = caps[1].parse::<f64>() {
                let percentage = (microseconds / 1_000_000.0 / duration) * 100.0;
                if report_progress {
                    DASHBOARD.update_progress(abstract_data.hash(), percentage);
                }
            }
        }
    }

    let status = child
        .wait()
        .context("Failed to wait for ffmpeg child process")?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "ffmpeg video compression failed with exit code {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(VideoCompressionOutcome::Video)
}
