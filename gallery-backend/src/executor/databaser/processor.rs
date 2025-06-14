use super::{
    fix_orientation::{fix_image_orientation, fix_image_width_height, fix_video_width_height},
    generate_dynamic_image::generate_dynamic_image,
    generate_exif::{generate_exif_for_image, generate_exif_for_video},
    generate_image_hash::{generate_phash, generate_thumbhash},
    generate_thumbnail::generate_thumbnail_for_image,
    generate_thumbnail::generate_thumbnail_for_video,
    generate_width_height::{generate_image_width_height, generate_video_width_height},
};
use crate::structure::database_struct::database::definition::Database;
use std::{error::Error, fs::metadata};

pub fn process_image_info(database: &mut Database) -> Result<(), Box<dyn Error>> {
    database.exif_vec = generate_exif_for_image(&database);
    let mut dynamic_image = generate_dynamic_image(&database)?;
    (database.width, database.height) = generate_image_width_height(&dynamic_image);
    fix_image_width_height(database);
    fix_image_orientation(database, &mut dynamic_image);
    database.thumbhash = generate_thumbhash(&dynamic_image)?;
    database.phash = generate_phash(&dynamic_image);
    generate_thumbnail_for_image(database, dynamic_image)?;
    Ok(())
}

pub fn process_video_info(database: &mut Database) -> Result<(), Box<dyn Error>> {
    database.exif_vec = generate_exif_for_video(&database)?;
    (database.width, database.height) = generate_video_width_height(&database)?;
    fix_video_width_height(database);
    generate_thumbnail_for_video(database)?;
    let dynamic_image = generate_dynamic_image(database)?;
    database.thumbhash = generate_thumbhash(&dynamic_image)?;
    database.phash = generate_phash(&dynamic_image);
    Ok(())
}

pub fn regenerate_metadata_for_image(database: &mut Database) -> Result<(), Box<dyn Error>> {
    database.size = metadata(&database.imported_path()).unwrap().len();
    process_image_info(database)?;
    Ok(())
}

pub fn regenerate_metadata_for_video(database: &mut Database) -> Result<(), Box<dyn Error>> {
    database.size = metadata(&database.imported_path()).unwrap().len();
    process_video_info(database)?;
    Ok(())
}
