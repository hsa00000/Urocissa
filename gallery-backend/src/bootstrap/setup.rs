use tokio::sync::mpsc::UnboundedReceiver;

use crate::background::processors::setup::{
    check_ffmpeg_and_ffprobe, initialize_file, initialize_folder, initialize_logger,
};

pub fn initialize() -> UnboundedReceiver<String> {
    let rx = initialize_logger();
    check_ffmpeg_and_ffprobe();
    initialize_folder();
    initialize_file();
    rx
}
