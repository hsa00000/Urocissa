use crate::public::structure::abstract_data::AbstractData;
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use std::{collections::BTreeMap, io, path::Path, process::Command, sync::LazyLock};

/// Extract EXIF metadata for images. On any failure, returns the original
/// map (possibly empty). Errors inside `read_exif` carry detailed context.
pub fn generate_exif_for_image(abstract_data: &AbstractData) -> BTreeMap<String, String> {
    generate_exif_for_image_path(&abstract_data.imported_path())
}

/// Extract image metadata from the canonical imported object. Reindexing must
/// never depend on an alias path because aliases may be removed after import.
pub fn generate_exif_for_image_path(file_path: &Path) -> BTreeMap<String, String> {
    let mut exif_tuple = BTreeMap::new();

    if let Ok(exif) = read_exif(file_path) {
        for field in exif.fields() {
            if field.ifd_num == exif::In::PRIMARY {
                let tag = field.tag.to_string();
                let value = field.display_value().with_unit(&exif).to_string();
                exif_tuple.insert(tag, value);
            }
        }
    }

    exif_tuple
}

/// Open the file, read EXIF data and attach *context* to every fallible step.
fn read_exif(file_path: &Path) -> Result<exif::Exif> {
    let exif_reader = exif::Reader::new();

    // Reading the file into a buffered reader
    let file = std::fs::File::open(file_path)
        .context(format!("failed to open file {}", file_path.display()))?;
    let mut bufreader = io::BufReader::with_capacity(1024 * 1024, &file);

    // Parsing EXIF data
    let exif = exif_reader
        .read_from_container(&mut bufreader)
        .context(format!(
            "failed to read EXIF metadata from {}",
            file_path.display()
        ))?;

    Ok(exif)
}

static RE_VIDEO_INFO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(.*?)=(.*?)\n").expect("regex compilation failure"));

/// Use `ffprobe` to retrieve metadata for videos, propagating every error
/// with rich context strings.
pub fn generate_exif_for_video(abstract_data: &AbstractData) -> Result<BTreeMap<String, String>> {
    generate_exif_for_video_path(&abstract_data.imported_path())
}

/// Probe the canonical imported video rather than its original alias.
pub fn generate_exif_for_video_path(file_path: &Path) -> Result<BTreeMap<String, String>> {
    let source_path = file_path.to_string_lossy();
    let mut exif_tuple = BTreeMap::new();

    // Spawn ffprobe and capture its output
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(file_path)
        .output()
        .context(format!("failed to spawn ffprobe for {source_path}"))?;

    if output.status.success() {
        // Convert raw bytes to UTF‑8 text
        let stdout = String::from_utf8(output.stdout).context(format!(
            "failed to convert ffprobe stdout to UTF‑8 for {source_path}"
        ))?;

        // Regex‑parse key/value pairs
        for cap in RE_VIDEO_INFO.captures_iter(&stdout) {
            let key = cap
                .get(1)
                .context(format!("capture group 1 missing in {source_path}"))?
                .as_str()
                .to_string();
            let value = cap
                .get(2)
                .context(format!("capture group 2 missing in {source_path}"))?
                .as_str()
                .to_string();
            exif_tuple.insert(key, value);
        }

        Ok(exif_tuple)
    } else {
        Err(anyhow!(
            "ffprobe exited with status {:?} for {}",
            output.status.code().unwrap_or(-1),
            source_path
        ))
    }
}
