//! The twenty-twenty library allows for visual regression testing of H264 frames and images.
#![deny(missing_docs)]

use anyhow::Result;

/// Convert a H264 frame to an image.
pub fn h264_frame_to_image(width: u32, height: u32, data: &[u8]) -> Result<image::DynamicImage> {
    let raw = if let Some(raw) = image::RgbImage::from_raw(width, height, data.to_vec()) {
        raw
    } else {
        anyhow::bail!("could not parse image from raw");
    };

    Ok(image::DynamicImage::ImageRgb8(raw))
}
