use crate::{
    public::constant::SHOULD_SWAP_WIDTH_HEIGHT_ROTATION,
    public::structure::abstract_data::AbstractData,
};
use image::DynamicImage;

pub fn fix_image_orientation(abstract_data: &AbstractData, dynamic_image: &mut DynamicImage) -> () {
    if let Some(exif_vec) = abstract_data.exif_vec() {
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
}

pub fn fix_image_width_height(abstract_data: &mut AbstractData) -> () {
    if let Some(exif_vec) = abstract_data.exif_vec() {
        if let Some(orientation) = exif_vec.get("Orientation") {
            match orientation.as_str() {
                "row 0 at right and column 0 at top" | "row 0 at left and column 0 at bottom" => {
                    abstract_data.swap_width_height();
                }
                _ => (),
            }
        }
    }
}

pub fn fix_video_width_height(abstract_data: &mut AbstractData) -> () {
    let should_swap = if let Some(exif_vec) = abstract_data.exif_vec() {
        if let Some(rotation) = exif_vec.get("rotation") {
            SHOULD_SWAP_WIDTH_HEIGHT_ROTATION.contains(&rotation.trim())
        } else {
            false
        }
    } else {
        false
    };
    if should_swap {
        abstract_data.swap_width_height();
    }
}
