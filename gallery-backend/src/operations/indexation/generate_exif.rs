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
    read_exif(file_path)
        .map(|exif| collect_primary_exif(&exif))
        .unwrap_or_default()
}

fn collect_primary_exif(exif: &exif::Exif) -> BTreeMap<String, String> {
    exif.fields()
        .filter(|field| field.ifd_num == exif::In::PRIMARY)
        .filter_map(|field| {
            format_exif_field(field, exif).map(|value| (field.tag.to_string(), value))
        })
        .collect()
}

fn format_exif_field(field: &exif::Field, exif: &exif::Exif) -> Option<String> {
    let value = normalize_exif_value(&field.value)?;
    let normalized_field = exif::Field {
        tag: field.tag,
        ifd_num: field.ifd_num,
        value,
    };
    let formatted = normalized_field.display_value().with_unit(exif).to_string();

    // Default ASCII rendering adds presentation quotes. Store the semantic
    // string while keeping specialized renderers such as DateTime and GPS.
    if let exif::Value::Ascii(values) = &normalized_field.value
        && let [value] = values.as_slice()
        && formatted.starts_with('"')
        && formatted.ends_with('"')
        && let Ok(value) = std::str::from_utf8(value)
    {
        return Some(value.to_owned());
    }

    Some(formatted)
}

fn normalize_exif_value(value: &exif::Value) -> Option<exif::Value> {
    match value {
        exif::Value::Ascii(values) => {
            // kamadak-exif exposes every NUL padding byte as an empty value.
            let values = values
                .iter()
                .filter(|value| value.iter().any(|byte| !byte.is_ascii_whitespace()))
                .cloned()
                .collect::<Vec<_>>();
            (!values.is_empty()).then_some(exif::Value::Ascii(values))
        }
        exif::Value::Undefined(bytes, _) if bytes.iter().all(|byte| *byte == 0) => None,
        value => Some(value.clone()),
    }
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

#[cfg(test)]
mod tests {
    use super::collect_primary_exif;

    struct TiffEntry {
        tag: u16,
        field_type: u16,
        count: u32,
        payload: Vec<u8>,
    }

    impl TiffEntry {
        fn ascii(tag: u16, value: &[u8], length: usize) -> Self {
            let mut payload = vec![0; length];
            payload[..value.len()].copy_from_slice(value);
            Self {
                tag,
                field_type: 2,
                count: length as u32,
                payload,
            }
        }

        fn undefined(tag: u16, payload: Vec<u8>) -> Self {
            Self {
                tag,
                field_type: 7,
                count: payload.len() as u32,
                payload,
            }
        }

        fn signed_rational(tag: u16, numerator: i32, denominator: i32) -> Self {
            let mut payload = Vec::with_capacity(8);
            payload.extend_from_slice(&numerator.to_le_bytes());
            payload.extend_from_slice(&denominator.to_le_bytes());
            Self {
                tag,
                field_type: 10,
                count: 1,
                payload,
            }
        }

        fn long(tag: u16, value: u32) -> Self {
            Self {
                tag,
                field_type: 4,
                count: 1,
                payload: value.to_le_bytes().to_vec(),
            }
        }
    }

    fn encoded_ifd_len(entries: &[TiffEntry]) -> usize {
        2 + entries.len() * 12
            + 4
            + entries
                .iter()
                .filter(|entry| entry.payload.len() > 4)
                .map(|entry| entry.payload.len())
                .sum::<usize>()
    }

    fn append_ifd(tiff: &mut Vec<u8>, entries: &[TiffEntry]) {
        let table_offset = tiff.len();
        let data_offset = table_offset + 2 + entries.len() * 12 + 4;
        tiff.resize(data_offset, 0);
        tiff[table_offset..table_offset + 2].copy_from_slice(&(entries.len() as u16).to_le_bytes());

        let mut next_payload_offset = data_offset;
        for (index, entry) in entries.iter().enumerate() {
            let entry_offset = table_offset + 2 + index * 12;
            tiff[entry_offset..entry_offset + 2].copy_from_slice(&entry.tag.to_le_bytes());
            tiff[entry_offset + 2..entry_offset + 4]
                .copy_from_slice(&entry.field_type.to_le_bytes());
            tiff[entry_offset + 4..entry_offset + 8].copy_from_slice(&entry.count.to_le_bytes());

            if entry.payload.len() <= 4 {
                tiff[entry_offset + 8..entry_offset + 8 + entry.payload.len()]
                    .copy_from_slice(&entry.payload);
            } else {
                tiff[entry_offset + 8..entry_offset + 12]
                    .copy_from_slice(&(next_payload_offset as u32).to_le_bytes());
                tiff.extend_from_slice(&entry.payload);
                next_payload_offset += entry.payload.len();
            }
        }
    }

    fn padded_exif_fixture() -> Vec<u8> {
        let mut ifd0 = vec![
            TiffEntry::ascii(0x010d, b"   ", 4),
            TiffEntry::ascii(0x010e, b"", 32),
            TiffEntry::ascii(0x010f, b"OPPO", 32),
            TiffEntry::ascii(0x0110, b"OPPO Reno6 Z 5G", 32),
            TiffEntry::ascii(0x0131, b"MediaTek Camera Application", 32),
            TiffEntry::ascii(0x0132, b"2023:03:29 18:42:05", 20),
            TiffEntry::ascii(0x013b, b"A\0B", 4),
            TiffEntry::ascii(0x8298, &[0xff], 2),
            TiffEntry::long(0x8769, 0),
        ];
        let exif_ifd_offset = 8 + encoded_ifd_len(&ifd0);
        ifd0.last_mut().expect("IFD0 should not be empty").payload =
            (exif_ifd_offset as u32).to_le_bytes().to_vec();

        let exif_ifd = vec![
            TiffEntry::ascii(0x9003, b"2023:03:29 18:42:05", 20),
            TiffEntry::ascii(0x9011, b"+08:00", 32),
            TiffEntry::signed_rational(0x9204, 0, 1),
            TiffEntry::undefined(
                0x927c,
                vec![0x7b, 0x22, 0x78, 0x22, 0x3a, 0x31, 0x7d, 0, 0, 0],
            ),
            TiffEntry::undefined(0x9286, vec![0; 16]),
        ];

        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"II");
        tiff.extend_from_slice(&42_u16.to_le_bytes());
        tiff.extend_from_slice(&8_u32.to_le_bytes());
        append_ifd(&mut tiff, &ifd0);
        assert_eq!(tiff.len(), exif_ifd_offset);
        append_ifd(&mut tiff, &exif_ifd);
        tiff
    }

    #[test]
    fn normalizes_padded_ascii_and_drops_empty_binary_metadata() {
        let exif = exif::Reader::new()
            .read_raw(padded_exif_fixture())
            .expect("fixture should contain valid EXIF data");
        let metadata = collect_primary_exif(&exif);

        assert_eq!(metadata.get("Make").map(String::as_str), Some("OPPO"));
        assert_eq!(
            metadata.get("Model").map(String::as_str),
            Some("OPPO Reno6 Z 5G")
        );
        assert_eq!(
            metadata.get("Software").map(String::as_str),
            Some("MediaTek Camera Application")
        );
        assert_eq!(
            metadata.get("OffsetTimeOriginal").map(String::as_str),
            Some("+08:00")
        );
        assert_eq!(
            metadata.get("DateTimeOriginal").map(String::as_str),
            Some("2023-03-29 18:42:05")
        );
        assert_eq!(
            metadata.get("ExposureBiasValue").map(String::as_str),
            Some("0 EV")
        );
        assert_eq!(
            metadata.get("Artist").map(String::as_str),
            Some("\"A\", \"B\"")
        );
        assert_eq!(
            metadata.get("Copyright").map(String::as_str),
            Some("\"\\xff\"")
        );
        assert!(!metadata.contains_key("DocumentName"));
        assert!(!metadata.contains_key("ImageDescription"));
        assert!(!metadata.contains_key("UserComment"));
        assert!(
            metadata
                .get("MakerNote")
                .is_some_and(|value| value.starts_with("0x7b2278223a317d"))
        );
    }
}
