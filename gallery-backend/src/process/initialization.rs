use tokio::sync::mpsc::UnboundedReceiver;

use crate::operations::initialization::{
    ffmpeg::check_ffmpeg_and_ffprobe, folder::initialize_folder, logger::initialize_logger,
    redb::initialize_file,
};
use crate::public::structure::{album::Album, database_struct::database::definition::Database};
use rusqlite::Connection;

pub fn initialize() -> UnboundedReceiver<String> {
    let rx = initialize_logger();
    check_ffmpeg_and_ffprobe();
    initialize_folder();
    initialize_file();
    rx
}
