use std::collections::BTreeMap;

use crate::{
    public::{
        constant::SHOULD_SWAP_WIDTH_HEIGHT_ROTATION,
        structure::database::definition::DatabaseSchema,
    },
    tasks::actor::index::IndexTask,
};
use image::DynamicImage;

pub fn fix_image_orientation(
    exif_vec: &BTreeMap<String, String>,
    dynamic_image: &mut DynamicImage,
) -> () {
    if let Some(orientation) = exif_vec.get("Orientation") {
        match orientation.as_str() {
            "row 0 at right and column 0 at top" => {
                *dynamic_image = dynamic_image.rotate90();
            }
            "row 0 at bottom and column 0 at right" => {
                *dynamic_image = dynamic_image.rotate180();
            }
            "row 0 at left and column 0 at bottom" => {
                *dynamic_image = dynamic_image.rotate270();
            }
            _ => (),
        }
    }
}

pub fn fix_image_width_height(index_task: &mut IndexTask) -> () {
    if let Some(orientation) = index_task.exif_vec.get("Orientation") {
        match orientation.as_str() {
            "row 0 at right and column 0 at top" => {
                std::mem::swap(&mut index_task.width, &mut index_task.height)
            }
            "row 0 at left and column 0 at bottom" => {
                std::mem::swap(&mut index_task.width, &mut index_task.height)
            }
            _ => (),
        }
    }
}

pub fn fix_video_width_height(index_task: &mut IndexTask) -> () {
    let should_swap_video_width_height = {
        if let Some(rotation) = index_task.exif_vec.get("rotation") {
            SHOULD_SWAP_WIDTH_HEIGHT_ROTATION.contains(&rotation.trim())
        } else {
            false
        }
    };
    if should_swap_video_width_height {
        (index_task.width, index_task.height) = (index_task.height, index_task.width)
    }
}
