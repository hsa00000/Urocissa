use crate::operations::initialization::{
    ffmpeg::check_ffmpeg_and_ffprobe, folder::initialize_folder, redb::initialize_file,
};

use crate::public::structure::config::AppConfig;

/// Initializes all core application subsystems.
pub fn initialize() {
    // Config must be initialized first to ensure 'config.json' exists for subsequent subsystems.
    AppConfig::init();

    // Ensure storage folders exist before trying to download FFmpeg into them
    initialize_folder();

    check_ffmpeg_and_ffprobe();
    initialize_file();
}
