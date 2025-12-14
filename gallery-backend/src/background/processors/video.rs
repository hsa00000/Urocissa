//! Video processing module - handles all video related logic.
//!
//! Includes:
//! - Video metadata extraction via ffprobe
//! - Thumbnail generation
//! - Video compression with progress monitoring
//! - Width/height calculation with rotation handling

use crate::{
    background::{
        actors::index::IndexTask,
        processors::image::{
            generate_dynamic_image, generate_dynamic_image_from_path, generate_phash,
            generate_thumbhash, small_width_height,
        },
    },
    cli::tui::DASHBOARD,
    common::SHOULD_SWAP_WIDTH_HEIGHT_ROTATION,
    database::schema::{
        image::ImageCombined,
        meta_image::ImageMetadataSchema,
        object::{ObjectSchema, ObjectType},
    },
    models::entity::abstract_data::AbstractData,
    utils::{compressed_path, imported_path, thumbnail_path},
};
use anyhow::{Context, Result, anyhow};
use arrayvec::ArrayString;
use log::info;
use regex::Regex;
use std::{
    cmp::min,
    collections::BTreeMap,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    sync::LazyLock,
};

use super::metadata::generate_exif_for_video;

// ────────────────────────────────────────────────────────────────
// Constants & Statics
// ────────────────────────────────────────────────────────────────

static REGEX_OUT_TIME_US: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"out_time_us=(\d+)").unwrap());

/// Heuristic duration (ms) often reported by FFmpeg for single-frame/static GIFs.
const STATIC_GIF_DURATION_MS: u32 = 100;
const MAX_VIDEO_HEIGHT: u32 = 720;
const FFMPEG_PROGRESS_PIPE: &str = "pipe:2";

// ────────────────────────────────────────────────────────────────
// Enums
// ────────────────────────────────────────────────────────────────

/// Result status after attempting to process a video file.
pub enum VideoProcessResult {
    Success,
    ConvertedToImage,
}

enum DurationAnalysis {
    Valid(f64),
    StaticOrCorruptGif,
    Error(anyhow::Error),
}

// ────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────

/// Analyzes the newly-imported video to populate metadata, dimensions, and hashes.
///
/// This orchestrates FFprobe extraction, rotation correction, and thumbnail generation.
pub fn process_video_info(index_task: &mut IndexTask) -> Result<()> {
    generate_exif_for_video(index_task).context("failed to extract video metadata via ffprobe")?;

    let (w, h) =
        generate_video_width_height(index_task).context("failed to obtain video width/height")?;
    index_task.width = w;
    index_task.height = h;

    fix_video_width_height(index_task);

    generate_thumbnail_for_video(index_task)
        .context("failed to generate video thumbnail via ffmpeg")?;

    let dynamic_image = generate_dynamic_image(index_task)
        .context("failed to decode first video frame into DynamicImage")?;

    index_task.thumbhash = generate_thumbhash(&dynamic_image);
    index_task.phash = generate_phash(&dynamic_image);

    Ok(())
}

/// Rebuilds all metadata and re-runs the processing pipeline for an existing video file.
pub fn regenerate_metadata_for_video(index_task: &mut IndexTask) -> Result<()> {
    index_task.size = std::fs::metadata(&index_task.imported_path)
        .context("failed to read metadata for imported video file")?
        .len();

    process_video_info(index_task).context("failed to process video info")?;
    Ok(())
}

/// Orchestrates the video compression workflow.
///
/// # Business Logic
/// Detects static GIFs (which are technically videos in some contexts) and converts
/// them to Image objects to avoid transcoding issues.
pub fn generate_compressed_video(data: &mut AbstractData) -> Result<VideoProcessResult> {
    let (id, ext, height, original_path, output_path) = match data {
        AbstractData::Video(v) => (
            v.object.id,
            v.metadata.ext.clone(),
            v.metadata.height,
            imported_path(&v.object.id, &v.metadata.ext),
            compressed_path(&v.object.id, ObjectType::Video),
        ),
        _ => return Err(anyhow!("Invalid data type: expected Video")),
    };

    let duration = match analyze_duration(&original_path, &ext) {
        DurationAnalysis::StaticOrCorruptGif => {
            info!(
                "Static or corrupt GIF detected. Processing as image: {:?}",
                original_path
            );
            convert_video_data_to_image_data(data)?;
            return Ok(VideoProcessResult::ConvertedToImage);
        }
        DurationAnalysis::Valid(d) => d,
        DurationAnalysis::Error(e) => return Err(e),
    };

    if let AbstractData::Video(v) = data {
        v.metadata.duration = duration;
    }

    compress_with_ffmpeg(&original_path, &output_path, height, duration, id)?;

    Ok(VideoProcessResult::Success)
}

// ────────────────────────────────────────────────────────────────
// Private Logic & Helpers
// ────────────────────────────────────────────────────────────────

fn analyze_duration(path: &Path, ext: &str) -> DurationAnalysis {
    match video_duration(&path.to_string_lossy()) {
        Ok(d) if (d * 1000.0) as u32 == STATIC_GIF_DURATION_MS => {
            DurationAnalysis::StaticOrCorruptGif
        }
        Ok(d) => DurationAnalysis::Valid(d),
        Err(e) => {
            // Fallback: Broken GIFs often fail ffprobe duration parsing entirely.
            if ext.eq_ignore_ascii_case("gif") && e.to_string().contains("fail to parse") {
                DurationAnalysis::StaticOrCorruptGif
            } else {
                DurationAnalysis::Error(e)
            }
        }
    }
}

fn compress_with_ffmpeg(
    input: &Path,
    output: &Path,
    height: u32,
    total_duration: f64,
    task_id: ArrayString<64>,
) -> Result<()> {
    // Constraint: Many video codecs (e.g., H.264) require dimensions to be divisible by 2.
    // We scale down if the video exceeds MAX_VIDEO_HEIGHT, ensuring even dimensions.
    let target_height = min(height, MAX_VIDEO_HEIGHT);
    let scale_filter = format!("scale=trunc(oh*a/2)*2:{}", (target_height / 2) * 2);

    let mut cmd = create_silent_ffmpeg_command();
    cmd.args([
        "-y",
        "-i",
        &input.to_string_lossy(),
        "-vf",
        &scale_filter,
        "-movflags",
        "faststart", // Optimizes for progressive web streaming (moves moov atom to front)
        &output.to_string_lossy(),
        "-progress",
        FFMPEG_PROGRESS_PIPE,
    ]);

    let mut child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn ffmpeg")?;

    if let Some(stderr) = child.stderr.take() {
        monitor_progress(stderr, total_duration, task_id);
    }

    child.wait().context("FFmpeg execution failed")?;
    Ok(())
}

fn monitor_progress(
    stderr: std::process::ChildStderr,
    total_duration: f64,
    task_id: ArrayString<64>,
) {
    let reader = BufReader::new(stderr);

    for line in reader.lines().filter_map(Result::ok) {
        if let Some(caps) = REGEX_OUT_TIME_US.captures(&line) {
            if let Ok(microseconds) = caps[1].parse::<f64>() {
                // (current_us / 1M) / total_seconds * 100
                let percentage = (microseconds / 1_000_000.0 / total_duration) * 100.0;
                DASHBOARD.update_progress(task_id, percentage);
            }
        }
    }
}

/// Mutates the AbstractData enum from Video variant to Image variant.
/// Used when a file initially identified as Video (e.g. GIF) is determined to be static.
fn convert_video_data_to_image_data(data: &mut AbstractData) -> Result<()> {
    let (video_combined, albums, tags) = match data {
        AbstractData::Video(v) => (v.clone(), v.albums.clone(), v.object.tags.clone()),
        _ => return Err(anyhow!("Data is not a video")),
    };

    let phash = generate_dynamic_image_from_path(&imported_path(
        &video_combined.object.id,
        &video_combined.metadata.ext,
    ))
    .ok()
    .map(|img| generate_phash(&img));

    let object = ObjectSchema {
        id: video_combined.object.id,
        obj_type: ObjectType::Image,
        created_time: video_combined.object.created_time,
        pending: false,
        thumbhash: video_combined.object.thumbhash,
        description: video_combined.object.description,
        tags,
        is_favorite: video_combined.object.is_favorite,
        is_archived: video_combined.object.is_archived,
        is_trashed: video_combined.object.is_trashed,
    };

    let metadata = ImageMetadataSchema {
        id: video_combined.metadata.id,
        size: video_combined.metadata.size,
        width: video_combined.metadata.width,
        height: video_combined.metadata.height,
        ext: video_combined.metadata.ext,
        phash,
    };

    let image_combined = ImageCombined {
        object,
        metadata,
        albums,
        exif_vec: BTreeMap::new(),
    };

    *data = AbstractData::Image(image_combined);
    Ok(())
}

// ────────────────────────────────────────────────────────────────
// Low-level FFmpeg Tools
// ────────────────────────────────────────────────────────────────

/// Generates a single JPEG thumbnail from the first frame (t=0).
pub fn generate_thumbnail_for_video(index_task: &mut IndexTask) -> Result<()> {
    let (width, height) = (index_task.width, index_task.height);
    let (thumb_width, thumb_height) = small_width_height(width, height, 1280);
    let thumbnail_path = thumbnail_path(index_task.hash());

    if let Some(parent) = Path::new(&thumbnail_path).parent() {
        std::fs::create_dir_all(parent)
            .context("failed to create parent directory for video thumbnail")?;
    }

    let mut cmd = create_silent_ffmpeg_command();
    cmd.args([
        "-y",
        "-i",
        &index_task.imported_path.to_string_lossy(),
        "-ss",
        "0",
        "-vframes",
        "1",
        "-vf",
        &format!("scale={}:{}", thumb_width, thumb_height),
        &thumbnail_path.to_string_lossy(),
    ]);

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

/// Probes the video file for logical width and height.
pub fn generate_video_width_height(index_task: &IndexTask) -> Result<(u32, u32)> {
    let imported = index_task.imported_path.to_string_lossy().to_string();

    let width = video_width_height("width", &imported)
        .context(format!("failed to obtain video width for {:?}", imported))?;
    let height = video_width_height("height", &imported)
        .context(format!("failed to obtain video height for {:?}", imported))?;

    Ok((width, height))
}

/// Swaps width and height in `index_task` if the EXIF rotation metadata implies a 90/270 degree turn.
pub fn fix_video_width_height(index_task: &mut IndexTask) {
    let should_swap = if let Some(rotation) = index_task.exif_vec.get("rotation") {
        SHOULD_SWAP_WIDTH_HEIGHT_ROTATION.contains(&rotation.trim())
    } else {
        false
    };

    if should_swap {
        (index_task.width, index_task.height) = (index_task.height, index_task.width)
    }
}

fn video_width_height(info: &str, file_path: &str) -> Result<u32> {
    let command_text = match info {
        "width" => "stream=width",
        "height" => "stream=height",
        _ => return Err(anyhow!("Invalid width/height probe command")),
    };

    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "error",
            "-show_entries",
            command_text,
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            file_path,
        ])
        .output()
        .context(format!("Fail to spawn ffprobe for {:?}", file_path))?;

    if output.status.success() {
        let val = String::from_utf8(output.stdout)?.trim().parse::<u32>()?;
        Ok(val)
    } else {
        Err(anyhow!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Uses ffprobe to extract the duration in seconds (f64).
pub fn video_duration(file_path: &str) -> Result<f64> {
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
            "Fail to spawn ffprobe for duration: {:?}",
            file_path
        ))?;

    if output.status.success() {
        let duration = String::from_utf8(output.stdout)?
            .trim()
            .parse::<f64>()
            .context(format!("Fail to parse duration to f64: {:?}", file_path))?;
        Ok(duration)
    } else {
        Err(anyhow!(
            "ffprobe duration check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn create_silent_ffmpeg_command() -> Command {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "quiet", "-hide_banner", "-nostats", "-nostdin"]);
    cmd
}
