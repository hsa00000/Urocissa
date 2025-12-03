//! Video processing module - handles all video related logic
//!
//! Includes:
//! - Video metadata extraction via ffprobe
//! - Thumbnail generation from first frame
//! - Video compression
//! - Width/height calculation with rotation handling

use crate::{
    public::{constant::SHOULD_SWAP_WIDTH_HEIGHT_ROTATION, tui::DASHBOARD, structure::abstract_data::Database},
    workflow::{
        processors::image::{
            generate_dynamic_image, generate_phash, generate_thumbhash, small_width_height,
        },
        tasks::actor::index::IndexTask,
    },
};
use anyhow::{Context, Result, anyhow};
use log::info;
use regex::Regex;
use std::{
    cmp,
    error::Error,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::LazyLock,
};

use super::metadata::generate_exif_for_video;

static REGEX_OUT_TIME_US: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"out_time_us=(\d+)").unwrap());

// ────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────

/// Analyse the newly-imported video and populate the DatabaseSchema record
pub fn process_video_info(index_task: &mut IndexTask) -> Result<()> {
    // Extract EXIF-like metadata via ffprobe
    generate_exif_for_video(index_task).context("failed to extract video metadata via ffprobe")?;

    // Get logical dimensions and fix if rotated
    (index_task.width, index_task.height) =
        generate_video_width_height(index_task).context("failed to obtain video width/height")?;
    fix_video_width_height(index_task);

    // Produce thumbnail from first frame
    generate_thumbnail_for_video(index_task)
        .context("failed to generate video thumbnail via ffmpeg")?;

    // Decode the first frame for hashing purposes
    let dynamic_image = generate_dynamic_image(index_task)
        .context("failed to decode first video frame into DynamicImage")?;

    // Compute perceptual hashes
    index_task.thumbhash = generate_thumbhash(&dynamic_image);
    index_task.phash = generate_phash(&dynamic_image);

    Ok(())
}

/// Rebuild all metadata for an existing video file
pub fn regenerate_metadata_for_video(index_task: &mut IndexTask) -> Result<()> {
    // Refresh size from filesystem metadata
    index_task.size = std::fs::metadata(index_task.imported_path())
        .context("failed to read metadata for imported video file")?
        .len();

    // Re-run the full processing pipeline
    process_video_info(index_task).context("failed to process video info")?;
    Ok(())
}

// ────────────────────────────────────────────────────────────────
// Width/Height Calculation
// ────────────────────────────────────────────────────────────────

/// Probe a video file using ffprobe to obtain `(width, height)`
pub fn generate_video_width_height(index_task: &IndexTask) -> Result<(u32, u32)> {
    let imported = index_task.imported_path_string();

    let width = video_width_height("width", &imported)
        .context(format!("failed to obtain video width for {:?}", imported))?;
    let height = video_width_height("height", &imported)
        .context(format!("failed to obtain video height for {:?}", imported))?;

    Ok((width, height))
}

pub fn fix_video_width_height(index_task: &mut IndexTask) {
    let should_swap_video_width_height = {
        if let Some(rotation) = index_task.exif_vec.get("rotation") {
            SHOULD_SWAP_WIDTH_HEIGHT_ROTATION.contains(&rotation.trim())
        } else {
            false
        }
    };
    if should_swap_video_width_height {
        (index_task.width, index_task.height) = (index_task.height, index_task.width)
    }
}

fn video_width_height(info: &str, file_path: &str) -> Result<u32> {
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

// ────────────────────────────────────────────────────────────────
// Thumbnail Generation
// ────────────────────────────────────────────────────────────────

/// Generate a single JPEG thumbnail taken from the first frame of a video asset.
/// Uses ffprobe for metadata and ffmpeg for frame extraction.
pub fn generate_thumbnail_for_video(index_task: &mut IndexTask) -> Result<()> {
    let (width, height) = (index_task.width, index_task.height);
    let (thumb_width, thumb_height) = small_width_height(width, height, 1280);
    let thumbnail_path = index_task.thumbnail_path();

    // Create target directory tree if missing
    std::fs::create_dir_all(index_task.compressed_path_parent())
        .context("failed to create parent directory for video thumbnail")?;

    // Assemble silent ffmpeg command
    let mut cmd = create_silent_ffmpeg_command();
    cmd.args([
        "-y",
        "-i",
        &index_task.imported_path_string(),
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

// ────────────────────────────────────────────────────────────────
// Video Compression
// ────────────────────────────────────────────────────────────────

/// Compresses a video file, reporting progress by parsing ffmpeg's output.
pub fn generate_compressed_video(database: &mut Database) -> Result<()> {
    let duration_result = video_duration(&database.imported_path_string());
    let duration = match duration_result {
        // Handle static GIFs by returning an error - should be processed as image instead
        Ok(d) if (d * 1000.0) as u32 == 100 => {
            info!(
                "Static GIF detected. Should be processed as image: {:?}",
                database.imported_path_string()
            );
            return Err(anyhow!("Static GIF should be processed as image"));
        }
        // Handle non-GIFs that fail to parse duration.
        Err(err)
            if err.to_string().contains("fail to parse to f32")
                && database.ext().eq_ignore_ascii_case("gif") =>
        {
            info!(
                "Potentially corrupt or non-standard GIF. Should be processed as image: {:?}",
                database.imported_path_string()
            );
            return Err(anyhow!("Corrupt GIF should be processed as image"));
        }
        Ok(d) => d,
        Err(err) => {
            return Err(anyhow::anyhow!(
                "Failed to get video duration for {:?}: {}",
                database.imported_path_string(),
                err
            ));
        }
    };

    let mut cmd = create_silent_ffmpeg_command();
    cmd.args([
        "-y", // Overwrite output file if it exists
        "-i",
        &database.imported_path_string(),
        "-vf",
        // Scale video to a max height of 720p, ensuring dimensions are even.
        &format!(
            "scale=trunc(oh*a/2)*2:{}",
            (cmp::min(database.height(), 720) / 2) * 2
        ),
        "-movflags",
        "faststart", // Optimize for web streaming
        &database.compressed_path_string(),
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
    for line in reader.lines().filter_map(Result::ok) {
        if let Some(caps) = REGEX_OUT_TIME_US.captures(&line) {
            // The regex captures the digits of the duration.
            // We only proceed if the captured value can be parsed as a number.
            if let Ok(microseconds) = caps[1].parse::<f64>() {
                let percentage = (microseconds / 1_000_000.0 / duration) * 100.0;
                DASHBOARD.update_progress(database.hash(), percentage);
            }
        }
    }

    child
        .wait()
        .context("Failed to wait for ffmpeg child process")?;
    Ok(())
}

// ────────────────────────────────────────────────────────────────
// FFmpeg/FFprobe Utilities
// ────────────────────────────────────────────────────────────────

/// Creates a base `ffmpeg` command with flags to ensure it runs silently.
/// This prevents duplicating arguments and ensures all ffmpeg calls are quiet.
pub fn create_silent_ffmpeg_command() -> Command {
    let mut cmd = Command::new("ffmpeg");
    // These global options must come before the input/output options.
    cmd.args(["-v", "quiet", "-hide_banner", "-nostats", "-nostdin"]);
    cmd
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
