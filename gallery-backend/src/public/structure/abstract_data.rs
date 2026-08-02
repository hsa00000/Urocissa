use std::collections::{BTreeMap, HashSet};
use std::fs::metadata;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::public::constant::VALID_IMAGE_EXTENSIONS;

use super::{
    album::AlbumCombined,
    common::FileModify,
    image::{ImageCombined, ImageMetadata},
    object::{ObjectSchema, ObjectType},
    video::{VideoCombined, VideoMetadata},
};

fn parse_filename_datetime(file_name: &str) -> Option<NaiveDateTime> {
    let [year, month, day, hour, minute, second] = first_filename_timestamp_parts(file_name)?;
    let date = NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day)?;
    let time = NaiveTime::from_hms_opt(hour, minute, second)?;
    Some(NaiveDateTime::new(date, time))
}

fn first_filename_timestamp_parts(file_name: &str) -> Option<[u32; 6]> {
    let bytes = file_name.as_bytes();
    let mut start = 0;
    let mut previous_is_word = false;
    while start < bytes.len() {
        let byte = bytes[start];
        if byte.is_ascii() {
            let is_word = byte.is_ascii_alphanumeric() || byte == b'_';
            if byte.is_ascii_digit()
                && !previous_is_word
                && let Some((parts, end)) = timestamp_parts_at(file_name, start)
                && word_boundary_after(file_name, end)
            {
                return Some(parts);
            }
            previous_is_word = is_word;
            start += 1;
        } else {
            let character = file_name[start..].chars().next()?;
            previous_is_word = regex_syntax::is_word_character(character);
            start += character.len_utf8();
        }
    }
    None
}

fn timestamp_parts_at(file_name: &str, start: usize) -> Option<([u32; 6], usize)> {
    let bytes = file_name.as_bytes();
    let mut cursor = start;
    let mut parts = [0_u32; 6];
    parts[0] = take_ascii_digits(bytes, &mut cursor, 4)?;
    for part in &mut parts[1..] {
        if !bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            let separator = file_name.get(cursor..)?.chars().next()?;
            if separator.is_ascii_alphanumeric() {
                return None;
            }
            cursor += separator.len_utf8();
        }
        *part = take_ascii_digits(bytes, &mut cursor, 2)?;
    }
    Some((parts, cursor))
}

fn take_ascii_digits(bytes: &[u8], cursor: &mut usize, count: usize) -> Option<u32> {
    let digits = bytes.get(*cursor..cursor.checked_add(count)?)?;
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    *cursor += count;
    Some(
        digits
            .iter()
            .fold(0_u32, |value, digit| value * 10 + u32::from(*digit - b'0')),
    )
}

fn word_boundary_after(value: &str, byte_index: usize) -> bool {
    let Some(&byte) = value.as_bytes().get(byte_index) else {
        return true;
    };
    if byte.is_ascii() {
        return !byte.is_ascii_alphanumeric() && byte != b'_';
    }
    value[byte_index..]
        .chars()
        .next()
        .is_none_or(|character| !regex_syntax::is_word_character(character))
}

pub fn thumbnail_file_name(hash: &str, cache_version: u32) -> String {
    if cache_version == 0 {
        format!("{hash}.jpg")
    } else {
        format!("{hash}-v{cache_version}.jpg")
    }
}

pub fn parse_thumbnail_stem(stem: &str) -> Option<(&str, u32)> {
    if stem.len() < 64 {
        return None;
    }
    let (hash, suffix) = stem.split_at(64);
    if !hash
        .as_bytes()
        .iter()
        .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(character))
    {
        return None;
    }
    if suffix.is_empty() {
        return Some((hash, 0));
    }
    let version_text = suffix.strip_prefix("-v")?;
    if version_text.starts_with('0') {
        return None;
    }
    let version = version_text.parse::<u32>().ok()?;
    Some((hash, version))
}

#[cfg(test)]
mod thumbnail_identity_tests {
    use super::{parse_thumbnail_stem, thumbnail_file_name};

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn thumbnail_names_keep_version_zero_backward_compatible() {
        assert_eq!(thumbnail_file_name(HASH, 0), format!("{HASH}.jpg"));
        assert_eq!(thumbnail_file_name(HASH, 42), format!("{HASH}-v42.jpg"));
    }

    #[test]
    fn thumbnail_stems_only_accept_canonical_versions() {
        assert_eq!(parse_thumbnail_stem(HASH), Some((HASH, 0)));
        assert_eq!(
            parse_thumbnail_stem(&format!("{HASH}-v42")),
            Some((HASH, 42))
        );
        for stem in [
            format!("{HASH}-v0"),
            format!("{HASH}-v01"),
            format!("{HASH}-v"),
            format!("{HASH}-other"),
            format!("{HASH}-v4294967296"),
        ] {
            assert_eq!(parse_thumbnail_stem(&stem), None);
        }
    }
}

#[cfg(test)]
mod filename_timestamp_tests {
    use std::hint::black_box;
    use std::sync::LazyLock;
    use std::time::{Duration, Instant};

    use chrono::TimeZone;
    use regex::Regex;

    use super::*;

    static LEGACY_FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b",
        )
        .unwrap()
    });

    fn legacy_parse_filename_datetime(file_name: &str) -> Option<NaiveDateTime> {
        let captures = LEGACY_FILE_NAME_TIME_REGEX.captures(file_name)?;
        let year = captures[1].parse::<i32>().ok()?;
        let month = captures[2].parse::<u32>().ok()?;
        let day = captures[3].parse::<u32>().ok()?;
        let hour = captures[4].parse::<u32>().ok()?;
        let minute = captures[5].parse::<u32>().ok()?;
        let second = captures[6].parse::<u32>().ok()?;
        Some(NaiveDateTime::new(
            NaiveDate::from_ymd_opt(year, month, day)?,
            NaiveTime::from_hms_opt(hour, minute, second)?,
        ))
    }

    #[test]
    fn allocation_free_filename_parser_matches_legacy_ascii_grammar() {
        let cases = [
            "20231225143052.jpg",
            "IMG-2023-12-25_14-30-52.jpg",
            "☃2023年12月25日14時30分52.jpg",
            "prefix 2023 12 25 14 30 52 suffix",
            "IMG_20231225_143052.jpg",
            "圖20231225143052.jpg",
            "20231225143052圖.jpg",
            "2023--12-25-14-30-52.jpg",
            "2023a12-25-14-30-52.jpg",
            "nothing-to-parse.jpg",
            "20240229112233.jpg",
            "20230229112233.jpg",
            "00000101000000.jpg",
        ];
        for case in cases {
            assert_eq!(
                parse_filename_datetime(case),
                legacy_parse_filename_datetime(case),
                "{case}"
            );
        }

        let separators = ["", "_", "-", ".", " ", "年", "☃"];
        let prefixes = ["", "IMG-", "IMG_", "圖", "☃", "x "];
        let suffixes = [".jpg", "-copy.jpg", "_copy.jpg", "圖.jpg", "☃.jpg"];
        let mut random = 0x7A17_51A9_2026_0718_u64;
        for index in 0..10_000_u32 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let year = 1990 + u32::try_from(random % 60).unwrap();
            let month = 1 + u32::try_from((random >> 8) % 14).unwrap();
            let day = 1 + u32::try_from((random >> 16) % 35).unwrap();
            let hour = u32::try_from((random >> 24) % 27).unwrap();
            let minute = u32::try_from((random >> 32) % 65).unwrap();
            let second = u32::try_from((random >> 40) % 65).unwrap();
            let separator = separators[usize::try_from(index).unwrap() % separators.len()];
            let prefix = prefixes
                [usize::try_from((random >> 5) % u64::try_from(prefixes.len()).unwrap()).unwrap()];
            let suffix = suffixes
                [usize::try_from((random >> 11) % u64::try_from(suffixes.len()).unwrap()).unwrap()];
            let case = format!(
                "{prefix}{year:04}{separator}{month:02}{separator}{day:02}{separator}{hour:02}{separator}{minute:02}{separator}{second:02}{suffix}"
            );
            assert_eq!(
                parse_filename_datetime(&case),
                legacy_parse_filename_datetime(&case),
                "{case}"
            );
        }
    }

    #[test]
    fn invalid_first_syntactic_match_blocks_later_filename_match() {
        let file_name = "20230230120000_then_20230301120000.jpg";
        assert_eq!(legacy_parse_filename_datetime(file_name), None);
        assert_eq!(parse_filename_datetime(file_name), None);
    }

    fn media_with_aliases(aliases: &[(&str, i64, i64)]) -> AbstractData {
        let id = ArrayString::<64>::from("filename-timestamp-test").unwrap();
        let mut metadata = ImageMetadata::new(id, 1, 1, 1, "jpg".to_owned());
        metadata.alias = aliases
            .iter()
            .map(|(file, modified, scan_time)| FileModify {
                file: (*file).to_owned(),
                modified: *modified,
                scan_time: *scan_time,
            })
            .collect();
        AbstractData::Image(ImageCombined {
            object: ObjectSchema::new(id, ObjectType::Image),
            metadata,
        })
    }

    #[test]
    fn filename_alias_future_and_priority_semantics_are_preserved() {
        let mut media = media_with_aliases(&[
            ("C:/gallery/20200102_030405.jpg", 900, 100),
            ("C:/gallery/20211231_235959.jpg", 800, 200),
        ]);
        let expected_filename = chrono::Local
            .from_local_datetime(
                &NaiveDate::from_ymd_opt(2021, 12, 31)
                    .unwrap()
                    .and_hms_opt(23, 59, 59)
                    .unwrap(),
            )
            .unwrap()
            .timestamp_millis();
        assert_eq!(media.compute_timestamp(&["filename"]), expected_filename);
        assert_eq!(media.compute_timestamp(&["scan_time"]), 200);
        assert_eq!(media.compute_timestamp(&["modified"]), 800);

        media.exif_vec_mut().unwrap().insert(
            "DateTimeOriginal".to_owned(),
            "2019-02-03 04:05:06".to_owned(),
        );
        let expected_exif = chrono::Local
            .from_local_datetime(
                &NaiveDate::from_ymd_opt(2019, 2, 3)
                    .unwrap()
                    .and_hms_opt(4, 5, 6)
                    .unwrap(),
            )
            .unwrap()
            .timestamp_millis();
        assert_eq!(
            media.compute_timestamp(&["DateTimeOriginal", "filename"]),
            expected_exif
        );

        let future = media_with_aliases(&[("C:/gallery/99991231_235959.jpg", 300, 400)]);
        assert_eq!(future.compute_timestamp(&["filename", "scan_time"]), 400);
        let invalid_first = media_with_aliases(&[(
            "C:/gallery/20230230120000_then_20230301120000.jpg",
            500,
            600,
        )]);
        assert_eq!(
            invalid_first.compute_timestamp(&["filename", "modified"]),
            500
        );
    }

    fn median(mut samples: Vec<Duration>) -> Duration {
        samples.sort_unstable();
        samples[samples.len() / 2]
    }

    fn measure<T>(mut operation: impl FnMut() -> T) -> Duration {
        for _ in 0..2 {
            black_box(operation());
        }
        median(
            (0..9)
                .map(|_| {
                    let started = Instant::now();
                    black_box(operation());
                    started.elapsed()
                })
                .collect(),
        )
    }

    #[test]
    #[ignore = "local 1M filename timestamp parser microbenchmark"]
    fn benchmark_million_fixture_filename_timestamps() {
        const ITEM_COUNT: usize = 1_000_000;
        let fixtures = (0..4_096_u64)
            .map(|index| {
                if index % 16 == 0 {
                    format!(
                        "IMG-2023-{:02}-{:02}_{:02}-{:02}-{:02}.jpg",
                        1 + index % 12,
                        1 + index % 28,
                        index % 24,
                        index % 60,
                        (index * 7) % 60
                    )
                } else {
                    let value = index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                    format!("{value:016x}{value:016x}{value:016x}{value:016x}.jpg")
                }
            })
            .collect::<Vec<_>>();

        let legacy = measure(|| {
            (0..ITEM_COUNT).fold(0_usize, |count, index| {
                count
                    + usize::from(
                        legacy_parse_filename_datetime(black_box(
                            &fixtures[index % fixtures.len()],
                        ))
                        .is_some(),
                    )
            })
        });
        let optimized = measure(|| {
            (0..ITEM_COUNT).fold(0_usize, |count, index| {
                count
                    + usize::from(
                        parse_filename_datetime(black_box(&fixtures[index % fixtures.len()]))
                            .is_some(),
                    )
            })
        });
        eprintln!(
            "filename timestamp legacy={legacy:?} optimized={optimized:?} speedup={:.2}x",
            legacy.as_secs_f64() / optimized.as_secs_f64()
        );
        assert!(optimized < legacy);
    }
}

/// `AbstractData` enum with Image, Video, and Album variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AbstractData {
    Image(ImageCombined),
    Video(VideoCombined),
    Album(AlbumCombined),
}

#[cfg(all(test, feature = "performance-test"))]
mod performance_tests {
    use super::AbstractData;

    #[test]
    fn performance_fixture_generation_is_deterministic_and_unique() {
        let first = AbstractData::generate_performance_data(17, 20_260_718);
        let repeat = AbstractData::generate_performance_data(17, 20_260_718);
        let next = AbstractData::generate_performance_data(18, 20_260_718);

        assert_eq!(first.hash(), repeat.hash());
        assert_eq!(first.source_path_string(), repeat.source_path_string());
        assert_ne!(first.hash(), next.hash());
        assert!(first.source_path_string().starts_with("perf://"));
    }

    #[test]
    fn performance_fixture_has_mixed_media_and_flags() {
        let items = (0..1_000)
            .map(|index| AbstractData::generate_performance_data(index, 20_260_718))
            .collect::<Vec<_>>();

        assert!(items.iter().any(AbstractData::is_image));
        assert!(items.iter().any(AbstractData::is_video));
        assert!(items.iter().any(AbstractData::is_favorite));
        assert!(items.iter().any(AbstractData::is_archived));
        assert!(items.iter().any(AbstractData::is_trashed));
    }
}

impl AbstractData {
    /// Get the object hash/id
    pub fn hash(&self) -> ArrayString<64> {
        match self {
            AbstractData::Image(img) => img.object.id,
            AbstractData::Video(vid) => vid.object.id,
            AbstractData::Album(alb) => alb.object.id,
        }
    }

    /// Get the width
    pub fn width(&self) -> u32 {
        match self {
            AbstractData::Image(img) => img.metadata.width,
            AbstractData::Video(vid) => vid.metadata.width,
            AbstractData::Album(_) => 300,
        }
    }

    /// Get the height
    pub fn height(&self) -> u32 {
        match self {
            AbstractData::Image(img) => img.metadata.height,
            AbstractData::Video(vid) => vid.metadata.height,
            AbstractData::Album(_) => 300,
        }
    }

    /// Get tags (reference)
    #[cfg(any(test, feature = "performance-test"))]
    pub fn tag(&self) -> &HashSet<String> {
        match self {
            AbstractData::Image(img) => &img.object.tags,
            AbstractData::Video(vid) => &vid.object.tags,
            AbstractData::Album(alb) => &alb.object.tags,
        }
    }

    /// Get tags (mutable reference)
    pub fn tag_mut(&mut self) -> &mut HashSet<String> {
        match self {
            AbstractData::Image(img) => &mut img.object.tags,
            AbstractData::Video(vid) => &mut vid.object.tags,
            AbstractData::Album(alb) => &mut alb.object.tags,
        }
    }

    /// Apply a stable mutation timestamp without changing it on overlay reads.
    pub fn touch_update_at(&mut self, changed_at: i64) {
        match self {
            AbstractData::Image(img) => img.object.touch_update_at(changed_at),
            AbstractData::Video(vid) => vid.object.touch_update_at(changed_at),
            AbstractData::Album(alb) => alb.object.touch_update_at(changed_at),
        }
    }

    /// Compute timestamp for sorting based on priority list
    /// Checks fields in order: `DateTimeOriginal`, filename, `scan_time`, modified, random
    pub fn compute_timestamp(&self, priority_list: &[&str]) -> i64 {
        if let AbstractData::Album(alb) = self {
            return alb.metadata.created_time;
        }

        let now_time = chrono::Local::now().naive_local();
        let exif_vec = self.exif_vec();
        let alias = self.alias();

        for &field in priority_list {
            match field {
                "DateTimeOriginal" => {
                    if let Some(exif) = exif_vec
                        && let Some(value) = exif.get("DateTimeOriginal")
                        && let Ok(naive_dt) =
                            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                        && let Some(local_dt) =
                            chrono::Local.from_local_datetime(&naive_dt).single()
                        && local_dt.naive_local() <= now_time
                    {
                        return local_dt.timestamp_millis();
                    }
                }
                "filename" => {
                    let mut max_time: Option<NaiveDateTime> = None;

                    for file_modify in alias {
                        if let Some(file_name) = Path::new(&file_modify.file).file_name()
                            && let Some(file_name_str) = file_name.to_str()
                            && let Some(datetime) = parse_filename_datetime(file_name_str)
                            && datetime <= now_time
                        {
                            max_time = Some(max_time.map_or(datetime, |t| t.max(datetime)));
                        }
                    }

                    if let Some(datetime) = max_time {
                        return chrono::Local
                            .from_local_datetime(&datetime)
                            .unwrap()
                            .timestamp_millis();
                    }
                }
                "scan_time" => {
                    let latest_scan_time = alias.iter().map(|a| a.scan_time).max();
                    if let Some(latest_time) = latest_scan_time {
                        return latest_time;
                    }
                }
                "modified" => {
                    if let Some(max_scan_alias) = alias.iter().max_by_key(|a| a.scan_time) {
                        return max_scan_alias.modified;
                    }
                }
                "random" => {
                    let mut rng = rand::rng();
                    let random_number: i64 = rng.random();
                    return random_number;
                }
                _ => panic!("Unknown field type: {field}"),
            }
        }
        0
    }

    /// Get `ext_type` (image/video/album)
    pub fn ext_type(&self) -> &str {
        match self {
            AbstractData::Image(_) => "image",
            AbstractData::Video(_) => "video",
            AbstractData::Album(_) => "album",
        }
    }

    /// Get file extension
    pub fn ext(&self) -> &str {
        match self {
            AbstractData::Image(img) => &img.metadata.ext,
            AbstractData::Video(vid) => &vid.metadata.ext,
            AbstractData::Album(_) => "",
        }
    }

    /// Get `exif_vec`
    pub fn exif_vec(&self) -> Option<&BTreeMap<String, String>> {
        match self {
            AbstractData::Image(img) => Some(&img.metadata.exif_vec),
            AbstractData::Video(vid) => Some(&vid.metadata.exif_vec),
            AbstractData::Album(_) => None,
        }
    }

    /// Get `exif_vec` mutable
    pub fn exif_vec_mut(&mut self) -> Option<&mut BTreeMap<String, String>> {
        match self {
            AbstractData::Image(img) => Some(&mut img.metadata.exif_vec),
            AbstractData::Video(vid) => Some(&mut vid.metadata.exif_vec),
            AbstractData::Album(_) => None,
        }
    }

    /// Get alias
    pub fn alias(&self) -> &[FileModify] {
        match self {
            AbstractData::Image(img) => &img.metadata.alias,
            AbstractData::Video(vid) => &vid.metadata.alias,
            AbstractData::Album(_) => &[],
        }
    }

    /// Get albums that this item belongs to
    pub fn albums(&self) -> Option<&HashSet<ArrayString<64>>> {
        match self {
            AbstractData::Image(img) => Some(&img.metadata.albums),
            AbstractData::Video(vid) => Some(&vid.metadata.albums),
            AbstractData::Album(_) => None,
        }
    }

    /// Get albums mutable
    pub fn albums_mut(&mut self) -> Option<&mut HashSet<ArrayString<64>>> {
        match self {
            AbstractData::Image(img) => Some(&mut img.metadata.albums),
            AbstractData::Video(vid) => Some(&mut vid.metadata.albums),
            AbstractData::Album(_) => None,
        }
    }

    /// Get thumbhash
    pub fn thumbhash(&self) -> Option<&Vec<u8>> {
        match self {
            AbstractData::Image(img) => img.object.thumbhash.as_ref(),
            AbstractData::Video(vid) => vid.object.thumbhash.as_ref(),
            AbstractData::Album(alb) => alb.object.thumbhash.as_ref(),
        }
    }

    /// Check if this is an image
    pub fn is_image(&self) -> bool {
        matches!(self, AbstractData::Image(_))
    }

    /// Check if this is a video
    pub fn is_video(&self) -> bool {
        matches!(self, AbstractData::Video(_))
    }

    #[cfg(feature = "performance-test")]
    pub fn is_favorite(&self) -> bool {
        match self {
            AbstractData::Image(data) => data.object.is_favorite,
            AbstractData::Video(data) => data.object.is_favorite,
            AbstractData::Album(data) => data.object.is_favorite,
        }
    }

    #[cfg(feature = "performance-test")]
    pub fn is_archived(&self) -> bool {
        match self {
            AbstractData::Image(data) => data.object.is_archived,
            AbstractData::Video(data) => data.object.is_archived,
            AbstractData::Album(data) => data.object.is_archived,
        }
    }

    #[cfg(feature = "performance-test")]
    pub fn is_trashed(&self) -> bool {
        match self {
            AbstractData::Image(data) => data.object.is_trashed,
            AbstractData::Video(data) => data.object.is_trashed,
            AbstractData::Album(data) => data.object.is_trashed,
        }
    }

    /// Create a new `AbstractData` from a file path and hash
    pub fn new(path: &Path, hash: ArrayString<64>) -> Result<Self> {
        let ext = path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("File has no extension: {}", path.display()))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Extension is not valid UTF-8: {}", path.display()))?
            .to_ascii_lowercase();

        let md = metadata(path)
            .with_context(|| format!("Failed to read metadata: {}", path.display()))?;
        let size = md.len();

        let modified_millis = md
            .modified()?
            .duration_since(UNIX_EPOCH)
            .with_context(|| format!("Modification time is before UNIX_EPOCH: {}", path.display()))?
            .as_millis();
        let modified_millis = i64::try_from(modified_millis).unwrap_or(0);

        let file_modify = FileModify::new(path, modified_millis);
        let obj_type = Self::determine_type(&ext);

        match obj_type {
            ObjectType::Image => {
                let object = ObjectSchema::new(hash, ObjectType::Image);
                let mut metadata = ImageMetadata::new(hash, size, 0, 0, ext);
                metadata.alias = vec![file_modify];
                Ok(AbstractData::Image(ImageCombined { object, metadata }))
            }
            ObjectType::Video => {
                let object = ObjectSchema::new(hash, ObjectType::Video);
                let mut metadata = VideoMetadata::new(hash, size, 0, 0, ext);
                metadata.alias = vec![file_modify];
                Ok(AbstractData::Video(VideoCombined { object, metadata }))
            }
            ObjectType::Album => Err(anyhow::anyhow!("Cannot create Album from file path")),
        }
    }

    fn determine_type(ext: &str) -> ObjectType {
        if VALID_IMAGE_EXTENSIONS.contains(&ext) {
            ObjectType::Image
        } else {
            ObjectType::Video
        }
    }

    /// Generate deterministic, metadata-only benchmark data.
    ///
    /// The benchmark deliberately does not create media files. The resulting
    /// records still exercise serialization, sorting, filtering, row layout,
    /// and deletion with a stable distribution across revisions.
    #[cfg(feature = "performance-test")]
    pub fn generate_performance_data(index: u64, seed: u64) -> Self {
        fn next(state: &mut u64) -> u64 {
            *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut value = *state;
            value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            value ^ (value >> 31)
        }

        let mut state = seed ^ index.wrapping_mul(0xD6E8_FEB8_6659_FD93);
        let hash = blake3::hash(format!("urocissa-perf:{seed}:{index}").as_bytes())
            .to_hex()
            .to_string();
        let hash = ArrayString::<64>::from(&hash).expect("BLAKE3 hash must fit ArrayString");
        let is_video = next(&mut state).is_multiple_of(10);
        let object_type = if is_video {
            ObjectType::Video
        } else {
            ObjectType::Image
        };
        let timestamp =
            946_684_800_000_i64 + i64::try_from(next(&mut state) % 820_454_400_000).unwrap_or(0);
        let width = 320 + u32::try_from(next(&mut state) % 1_920).unwrap_or(0);
        let height = 240 + u32::try_from(next(&mut state) % 1_080).unwrap_or(0);
        let tags = (0..usize::try_from(next(&mut state) % 5).unwrap_or(0))
            .map(|tag_index| format!("perf-tag-{}", (next(&mut state) % 32) + tag_index as u64))
            .collect::<HashSet<_>>();
        let file_modify = FileModify {
            file: format!("perf://{hash}"),
            modified: timestamp,
            scan_time: timestamp,
        };
        let object = ObjectSchema {
            id: hash,
            obj_type: object_type,
            pending: false,
            thumbhash: None,
            cache_version: 0,
            description: next(&mut state)
                .is_multiple_of(5)
                .then(|| format!("Performance fixture item {index}")),
            tags,
            is_favorite: next(&mut state).is_multiple_of(10),
            is_archived: next(&mut state).is_multiple_of(20),
            is_trashed: next(&mut state).is_multiple_of(50),
            update_at: timestamp,
        };

        if is_video {
            AbstractData::Video(VideoCombined {
                object,
                metadata: VideoMetadata {
                    id: hash,
                    size: 1_000 + next(&mut state) % 20_000_000,
                    width,
                    height,
                    ext: "mp4".to_string(),
                    duration: f64::from(
                        u16::try_from(next(&mut state) % 3_600).expect("bounded duration fits u16"),
                    ) + 0.5,
                    albums: HashSet::new(),
                    exif_vec: BTreeMap::new(),
                    alias: vec![file_modify],
                },
            })
        } else {
            AbstractData::Image(ImageCombined {
                object,
                metadata: ImageMetadata {
                    id: hash,
                    size: 1_000 + next(&mut state) % 20_000_000,
                    width,
                    height,
                    ext: "jpg".to_string(),
                    phash: None,
                    albums: HashSet::new(),
                    exif_vec: BTreeMap::new(),
                    alias: vec![file_modify],
                },
            })
        }
    }

    // Path helper methods

    /// Get the source path string (first alias)
    pub fn source_path_string(&self) -> &str {
        match self {
            AbstractData::Image(img) => &img.metadata.alias[0].file,
            AbstractData::Video(vid) => &vid.metadata.alias[0].file,
            AbstractData::Album(_) => "",
        }
    }

    /// Get the source path
    pub fn source_path(&self) -> PathBuf {
        PathBuf::from(self.source_path_string())
    }

    /// Get the imported path string
    pub fn imported_path_string(&self) -> String {
        let hash = self.hash();
        let ext = self.ext();
        crate::public::constant::storage::get_data_path()
            .join(format!(
                "object/imported/{}/{}.{}",
                &hash.as_str()[0..2],
                hash,
                ext
            ))
            .to_string_lossy()
            .into_owned()
    }

    /// Get the compressed path string
    pub fn compressed_path_string(&self) -> String {
        let hash = self.hash();
        let relative_path = match self {
            AbstractData::Image(_) => format!(
                "object/compressed/{}/{}",
                &hash.as_str()[0..2],
                thumbnail_file_name(hash.as_str(), self.cache_version())
            ),
            AbstractData::Video(_) => {
                format!("object/compressed/{}/{}.mp4", &hash.as_str()[0..2], hash)
            }
            AbstractData::Album(_) => String::new(),
        };

        if relative_path.is_empty() {
            return String::new();
        }

        crate::public::constant::storage::get_data_path()
            .join(relative_path)
            .to_string_lossy()
            .into_owned()
    }

    /// Get the imported path
    pub fn imported_path(&self) -> PathBuf {
        PathBuf::from(self.imported_path_string())
    }

    /// Get the compressed path
    pub fn compressed_path(&self) -> PathBuf {
        PathBuf::from(self.compressed_path_string())
    }

    /// Get the thumbnail path
    pub fn thumbnail_path(&self) -> String {
        self.thumbnail_path_for_version(self.cache_version())
            .to_string_lossy()
            .into_owned()
    }

    /// Get the thumbnail path for a specific immutable cache version.
    pub fn thumbnail_path_for_version(&self, cache_version: u32) -> PathBuf {
        let hash = self.hash();
        crate::public::constant::storage::get_data_path().join(format!(
            "object/compressed/{}/{}",
            &hash.as_str()[0..2],
            thumbnail_file_name(hash.as_str(), cache_version)
        ))
    }

    /// Get the parent directory of the compressed path
    pub fn compressed_path_parent(&self) -> PathBuf {
        self.compressed_path()
            .parent()
            .expect("Path::new(&output_file_path_string).parent() fail")
            .to_path_buf()
    }

    /// Get mutable alias
    pub fn alias_mut(&mut self) -> Option<&mut Vec<FileModify>> {
        match self {
            AbstractData::Image(img) => Some(&mut img.metadata.alias),
            AbstractData::Video(vid) => Some(&mut vid.metadata.alias),
            AbstractData::Album(_) => None,
        }
    }

    /// Set pending status
    pub fn set_pending(&mut self, pending: bool) {
        match self {
            AbstractData::Image(img) => img.object.pending = pending,
            AbstractData::Video(vid) => vid.object.pending = pending,
            AbstractData::Album(alb) => alb.object.pending = pending,
        }
    }

    /// Set favorite status
    pub fn set_favorite(&mut self, is_favorite: bool) {
        match self {
            AbstractData::Image(img) => img.object.is_favorite = is_favorite,
            AbstractData::Video(vid) => vid.object.is_favorite = is_favorite,
            AbstractData::Album(alb) => alb.object.is_favorite = is_favorite,
        }
    }

    /// Set archived status
    pub fn set_archived(&mut self, is_archived: bool) {
        match self {
            AbstractData::Image(img) => img.object.is_archived = is_archived,
            AbstractData::Video(vid) => vid.object.is_archived = is_archived,
            AbstractData::Album(alb) => alb.object.is_archived = is_archived,
        }
    }

    /// Set trashed status
    pub fn set_trashed(&mut self, is_trashed: bool) {
        match self {
            AbstractData::Image(img) => img.object.is_trashed = is_trashed,
            AbstractData::Video(vid) => vid.object.is_trashed = is_trashed,
            AbstractData::Album(alb) => alb.object.is_trashed = is_trashed,
        }
    }

    /// Get mutable reference to width
    pub fn set_width(&mut self, width: u32) {
        match self {
            AbstractData::Image(img) => img.metadata.width = width,
            AbstractData::Video(vid) => vid.metadata.width = width,
            AbstractData::Album(_) => {}
        }
    }

    /// Get mutable reference to height
    pub fn set_height(&mut self, height: u32) {
        match self {
            AbstractData::Image(img) => img.metadata.height = height,
            AbstractData::Video(vid) => vid.metadata.height = height,
            AbstractData::Album(_) => {}
        }
    }

    /// Swap width and height
    pub fn swap_width_height(&mut self) {
        match self {
            AbstractData::Image(img) => {
                std::mem::swap(&mut img.metadata.width, &mut img.metadata.height);
            }
            AbstractData::Video(vid) => {
                std::mem::swap(&mut vid.metadata.width, &mut vid.metadata.height);
            }
            AbstractData::Album(_) => {}
        }
    }

    /// Set thumbhash
    pub fn set_thumbhash(&mut self, thumbhash: Vec<u8>) {
        match self {
            AbstractData::Image(img) => img.object.thumbhash = Some(thumbhash),
            AbstractData::Video(vid) => vid.object.thumbhash = Some(thumbhash),
            AbstractData::Album(alb) => alb.object.thumbhash = Some(thumbhash),
        }
    }

    /// Get the immutable thumbnail cache version.
    pub fn cache_version(&self) -> u32 {
        match self {
            AbstractData::Image(img) => img.object.cache_version,
            AbstractData::Video(vid) => vid.object.cache_version,
            AbstractData::Album(alb) => alb.object.cache_version,
        }
    }

    /// Set the immutable thumbnail cache version.
    pub fn set_cache_version(&mut self, cache_version: u32) {
        match self {
            AbstractData::Image(img) => img.object.cache_version = cache_version,
            AbstractData::Video(vid) => vid.object.cache_version = cache_version,
            AbstractData::Album(alb) => alb.object.cache_version = cache_version,
        }
    }

    /// Set phash (only for images)
    pub fn set_phash(&mut self, phash: Vec<u8>) {
        if let AbstractData::Image(img) = self {
            img.metadata.phash = Some(phash);
        }
    }

    /// Set size
    pub fn set_size(&mut self, size: u64) {
        match self {
            AbstractData::Image(img) => img.metadata.size = size,
            AbstractData::Video(vid) => vid.metadata.size = size,
            AbstractData::Album(_) => {}
        }
    }

    /// Convert to Image type (if currently video)
    pub fn convert_to_image(&mut self) {
        if let AbstractData::Video(vid) = self {
            let object = ObjectSchema {
                id: vid.object.id,
                obj_type: ObjectType::Image,
                pending: vid.object.pending,
                thumbhash: vid.object.thumbhash.clone(),
                cache_version: vid.object.cache_version,
                description: vid.object.description.clone(),
                tags: vid.object.tags.clone(),
                is_favorite: vid.object.is_favorite,
                is_archived: vid.object.is_archived,
                is_trashed: vid.object.is_trashed,
                update_at: vid.object.update_at,
            };
            let metadata = ImageMetadata {
                id: vid.metadata.id,
                size: vid.metadata.size,
                width: vid.metadata.width,
                height: vid.metadata.height,
                ext: vid.metadata.ext.clone(),
                phash: None,
                albums: vid.metadata.albums.clone(),
                exif_vec: vid.metadata.exif_vec.clone(),
                alias: vid.metadata.alias.clone(),
            };
            *self = AbstractData::Image(ImageCombined { object, metadata });
        }
    }
}

impl From<ImageCombined> for AbstractData {
    fn from(image: ImageCombined) -> Self {
        AbstractData::Image(image)
    }
}

impl From<VideoCombined> for AbstractData {
    fn from(video: VideoCombined) -> Self {
        AbstractData::Video(video)
    }
}

impl From<AlbumCombined> for AbstractData {
    fn from(album: AlbumCombined) -> Self {
        AbstractData::Album(album)
    }
}
