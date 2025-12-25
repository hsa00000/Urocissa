use std::collections::{BTreeMap, HashSet};
use std::fs::metadata;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use arrayvec::ArrayString;
use bitcode::{Decode, Encode};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::public::constant::VALID_IMAGE_EXTENSIONS;

/// Regex for parsing timestamps from filenames (e.g., 20231225_143052)
static FILE_NAME_TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})[^a-zA-Z0-9]?(\d{2})\b").unwrap()
});

use super::{
    album::AlbumCombined,
    common::FileModify,
    image::{ImageCombined, ImageMetadata},
    object::{ObjectSchema, ObjectType},
    video::{VideoCombined, VideoMetadata},
};

/// AbstractData enum with Image, Video, and Album variants
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AbstractData {
    Image(ImageCombined),
    Video(VideoCombined),
    Album(AlbumCombined),
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

    /// Compute timestamp for sorting based on priority list
    /// Checks fields in order: DateTimeOriginal, filename, scan_time, modified, random
    pub fn compute_timestamp(&self, priority_list: &[&str]) -> u128 {
        if let AbstractData::Album(alb) = self {
            return alb.metadata.created_time as u128;
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
                        return local_dt.timestamp_millis() as u128;
                    }
                }
                "filename" => {
                    let mut max_time: Option<NaiveDateTime> = None;

                    for file_modify in alias {
                        let path = PathBuf::from(&file_modify.file);

                        if let Some(file_name) = path.file_name()
                            && let Some(file_name_str) = file_name.to_str()
                            && let Some(caps) = FILE_NAME_TIME_REGEX.captures(file_name_str)
                            && let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(minute), Ok(second)) = (
                                caps[1].parse::<i32>(),
                                caps[2].parse::<u32>(),
                                caps[3].parse::<u32>(),
                                caps[4].parse::<u32>(),
                                caps[5].parse::<u32>(),
                                caps[6].parse::<u32>(),
                            )
                            && let Some(date) = NaiveDate::from_ymd_opt(year, month, day)
                            && let Some(time) = NaiveTime::from_hms_opt(hour, minute, second)
                        {
                            let datetime = NaiveDateTime::new(date, time);

                            if datetime <= now_time {
                                max_time = Some(max_time.map_or(datetime, |t| t.max(datetime)));
                            }
                        }
                    }

                    if let Some(datetime) = max_time {
                        return chrono::Local
                            .from_local_datetime(&datetime)
                            .unwrap()
                            .timestamp_millis() as u128;
                    }
                }
                "scan_time" => {
                    let latest_scan_time = alias.iter().map(|a| a.scan_time).max();
                    if let Some(latest_time) = latest_scan_time {
                        return latest_time as u128;
                    }
                }
                "modified" => {
                    if let Some(max_scan_alias) = alias.iter().max_by_key(|a| a.scan_time) {
                        return max_scan_alias.modified;
                    }
                }
                "random" => {
                    let mut rng = rand::rng();
                    let random_number: u128 = rng.random();
                    return random_number;
                }
                _ => panic!("Unknown field type: {}", field),
            }
        }
        0
    }

    /// Get ext_type (image/video/album)
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

    /// Get exif_vec
    pub fn exif_vec(&self) -> Option<&BTreeMap<String, String>> {
        match self {
            AbstractData::Image(img) => Some(&img.metadata.exif_vec),
            AbstractData::Video(vid) => Some(&vid.metadata.exif_vec),
            AbstractData::Album(_) => None,
        }
    }

    /// Get exif_vec mutable
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

    /// Create a new AbstractData from a file path and hash
    pub fn new(path: &Path, hash: ArrayString<64>) -> Result<Self> {
        let ext = path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("File has no extension: {:?}", path))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Extension is not valid UTF-8: {:?}", path))?
            .to_ascii_lowercase();

        let md = metadata(path).with_context(|| format!("Failed to read metadata: {:?}", path))?;
        let size = md.len();

        let modified_millis = md
            .modified()?
            .duration_since(UNIX_EPOCH)
            .with_context(|| format!("Modification time is before UNIX_EPOCH: {:?}", path))?
            .as_millis();

        let file_modify = FileModify::new(path, modified_millis);
        let obj_type = Self::determine_type(&ext);

        match obj_type {
            ObjectType::Image => {
                let object = ObjectSchema::new(hash, ObjectType::Image);
                let mut metadata = ImageMetadata::new(hash, size, 0, 0, ext);
                metadata.alias = vec![file_modify];
                Ok(AbstractData::Image(ImageCombined {
                    object,
                    metadata,
                }))
            }
            ObjectType::Video => {
                let object = ObjectSchema::new(hash, ObjectType::Video);
                let mut metadata = VideoMetadata::new(hash, size, 0, 0, ext);
                metadata.alias = vec![file_modify];
                Ok(AbstractData::Video(VideoCombined {
                    object,
                    metadata,
                }))
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

    /// Generate random AbstractData (image) for testing
    pub fn generate_random_data() -> Self {
        use crate::operations::hash::generate_random_hash;
        use rand::Rng;

        let hash = generate_random_hash();
        let width = rand::rng().random_range(300..=600);
        let height = rand::rng().random_range(300..=600);

        let object = ObjectSchema {
            id: hash,
            obj_type: ObjectType::Image,
            pending: false,
            thumbhash: None,
            description: None,
            tags: HashSet::new(),
            is_favorite: false,
            is_archived: false,
            is_trashed: false,
        };

        let metadata = ImageMetadata {
            id: hash,
            size: 0,
            width,
            height,
            ext: "jpg".to_string(),
            phash: None,
            albums: HashSet::new(),
            exif_vec: BTreeMap::new(),
            alias: vec![FileModify {
                file: String::from("/"),
                modified: 0,
                scan_time: 0,
            }],
        };

        AbstractData::Image(ImageCombined {
            object,
            metadata,
        })
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
        format!(
            "./object/imported/{}/{}.{}",
            &hash.as_str()[0..2],
            hash,
            ext
        )
    }

    /// Get the compressed path string
    pub fn compressed_path_string(&self) -> String {
        let hash = self.hash();
        match self {
            AbstractData::Image(_) => {
                format!("./object/compressed/{}/{}.jpg", &hash.as_str()[0..2], hash)
            }
            AbstractData::Video(_) => {
                format!("./object/compressed/{}/{}.mp4", &hash.as_str()[0..2], hash)
            }
            AbstractData::Album(_) => String::new(),
        }
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
        let hash = self.hash();
        format!("./object/compressed/{}/{}.jpg", &hash.as_str()[0..2], hash)
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
                description: vid.object.description.clone(),
                tags: vid.object.tags.clone(),
                is_favorite: vid.object.is_favorite,
                is_archived: vid.object.is_archived,
                is_trashed: vid.object.is_trashed,
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
            *self = AbstractData::Image(ImageCombined {
                object,
                metadata,
            });
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
