use tokio::sync::mpsc::UnboundedReceiver;

use crate::operations::initialization::{
    ffmpeg::check_ffmpeg_and_ffprobe, folder::initialize_folder, logger::initialize_logger,
};
use crate::public::db::tree::initialize_from_db;

pub fn initialize() -> UnboundedReceiver<String> {
    let rx = initialize_logger();
    check_ffmpeg_and_ffprobe();
    initialize_folder();
    initialize_from_db();
    rx
}

