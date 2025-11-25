use std::collections::BTreeMap;

use crate::{
    public::constant::SHOULD_SWAP_WIDTH_HEIGHT_ROTATION,
    public::structure::database::definition::DatabaseSchema,
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

pub fn fix_image_width_height(
    database: &mut DatabaseSchema,
    exif_vec: &BTreeMap<String, String>,
) -> () {
    if let Some(orientation) = exif_vec.get("Orientation") {
        match orientation.as_str() {
            "row 0 at right and column 0 at top" => {
                std::mem::swap(&mut database.width, &mut database.height)
            }
            "row 0 at left and column 0 at bottom" => {
                std::mem::swap(&mut database.width, &mut database.height)
            }
            _ => (),
        }
    }
}

pub fn fix_video_width_height(
    database: &mut DatabaseSchema,
    exif_vec: &BTreeMap<String, String>,
) -> () {
    let should_swap_video_width_height = {
        if let Some(rotation) = exif_vec.get("rotation") {
            SHOULD_SWAP_WIDTH_HEIGHT_ROTATION.contains(&rotation.trim())
        } else {
            false
        }
    };
    if should_swap_video_width_height {
        (database.width, database.height) = (database.height, database.width)
    }
}
