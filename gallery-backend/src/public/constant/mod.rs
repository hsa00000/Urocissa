pub mod runtime;
pub mod storage;

pub const ROW_BATCH_NUMBER: usize = 20;

pub const SNAPSHOT_MAX_LIFETIME_MS: u64 = 24 * 60 * 60 * 1_000; // 24 hours

pub const MAX_COPY_RETRIES: u32 = 3;

pub const MAX_DELETE_ATTEMPTS: u64 = 5;

pub const SHOULD_SWAP_WIDTH_HEIGHT_ROTATION: &[&str] = &["90", "-90", "270", "-270"];

pub const VALID_IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "jfif", "jpe", "png", "tif", "tiff", "webp", "bmp",
];

pub const VALID_VIDEO_EXTENSIONS: &[&str] = &[
    "gif", "mp4", "webm", "mkv", "mov", "avi", "flv", "wmv", "mpeg",
];

pub const DEFAULT_PRIORITY_LIST: &[&str] =
    &["DateTimeOriginal", "filename", "modified", "scan_time"];
