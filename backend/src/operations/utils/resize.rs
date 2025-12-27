/// Resize dimensions so that the larger side equals `small_height`, preserving aspect ratio.
///
/// # Parameters
/// - `width`: original width of the image.
/// - `height`: original height of the image.
/// - `small_height`: target size for the larger side.
///
/// # Returns
/// A tuple `(new_width, new_height)` representing the scaled dimensions.
pub fn small_width_height(width: u32, height: u32, small_height: u32) -> (u32, u32) {
    // If width is the largest dimension (and larger than target), scale width
    let (nwidth, nheight) = if width >= std::cmp::max(height, small_height) {
        (small_height, height * small_height / width)
    // Else if height is the largest dimension (and larger than target), scale height
    } else if height >= std::cmp::max(width, small_height) {
        (width * small_height / height, small_height)
    // Otherwise, both dimensions are within the target size: no scaling
    } else {
        (width, height)
    };

    (nwidth, nheight)
}
