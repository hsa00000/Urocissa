use image::DynamicImage;
use image_hasher::HasherConfig;
pub fn generate_thumbhash(dynamic_image_rotated: &DynamicImage) -> Vec<u8> {
    let resized_image = dynamic_image_rotated.thumbnail_exact(100, 100);
    let rgba_image = resized_image.to_rgba8();
    let (swidth, sheight) = (rgba_image.width(), rgba_image.height());
    let thumbhash = thumbhash::rgba_to_thumb_hash(swidth as usize, sheight as usize, &rgba_image);
    thumbhash
}

pub fn generate_phash(dynamic_image_rotated: &DynamicImage) -> Vec<u8> {
    let hasher = HasherConfig::new().to_hasher();
    let phash = hasher.hash_image(dynamic_image_rotated);
    phash.as_bytes().to_vec()
}
