/// Resize dimensions so that the smaller side equals `target_short_side`, preserving aspect ratio.
///
/// This function ensures that the shortest side of the image is scaled down to `target_short_side`
/// if it exceeds that value. If the shortest side is already smaller than or equal to
/// `target_short_side`, the dimensions remain unchanged.
///
/// # Parameters
/// - `width`: original width of the image.
/// - `height`: original height of the image.
/// - `target_short_side`: the maximum allowed size for the smaller side of the image.
///
/// # Returns
/// A tuple `(new_width, new_height)` representing the scaled dimensions.
pub fn small_width_height(width: u32, height: u32, target_short_side: u32) -> (u32, u32) {
    // Identify the length of the smaller side of the original image
    let min_dimension = std::cmp::min(width, height);

    // Only scale if the smaller side is larger than the target limit
    if min_dimension > target_short_side {
        if width < height {
            // Width is the smaller side (Portrait or Landscape where width < height isn't standard, but logically valid)
            // Scale width to target, calculate height proportionally
            // Formula: new_height = original_height * (target / original_width)
            (target_short_side, height * target_short_side / width)
        } else {
            // Height is the smaller side (Landscape or Square)
            // Scale height to target, calculate width proportionally
            // Formula: new_width = original_width * (target / original_height)
            (width * target_short_side / height, target_short_side)
        }
    } else {
        // The image's smaller side is within the limit, return original dimensions
        (width, height)
    }
}

/// Resize dimensions so that the longer side is at most `max_long_side`, preserving aspect ratio.
/// Dimensions that already fit are returned unchanged.
pub fn max_long_side_width_height(width: u32, height: u32, max_long_side: u32) -> (u32, u32) {
    let long_side = width.max(height);

    if long_side <= max_long_side {
        return (width, height);
    }

    if max_long_side == 0 {
        return (0, 0);
    }

    let scale_dimension = |dimension: u32| {
        let numerator = u64::from(dimension) * u64::from(max_long_side);
        let denominator = u64::from(long_side);
        let rounded = (numerator + denominator / 2) / denominator;

        u32::try_from(rounded).unwrap_or(max_long_side).max(1)
    };

    (scale_dimension(width), scale_dimension(height))
}

#[cfg(test)]
mod tests {
    use super::{max_long_side_width_height, small_width_height};

    #[test]
    fn constrains_landscape_image_by_long_side() {
        assert_eq!(max_long_side_width_height(4000, 3000, 1920), (1920, 1440));
    }

    #[test]
    fn constrains_portrait_image_by_long_side() {
        assert_eq!(max_long_side_width_height(3000, 4000, 1920), (1440, 1920));
    }

    #[test]
    fn does_not_upscale_image_that_already_fits() {
        assert_eq!(max_long_side_width_height(1600, 900, 1920), (1600, 900));
    }

    #[test]
    fn keeps_extreme_aspect_ratio_dimension_nonzero() {
        assert_eq!(max_long_side_width_height(10_000, 1, 1920), (1920, 1));
    }

    #[test]
    fn short_side_resize_behavior_remains_available_for_video_thumbnails() {
        assert_eq!(small_width_height(4000, 3000, 1280), (1706, 1280));
    }
}
