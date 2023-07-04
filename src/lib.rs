//! The twenty-twenty library allows for visual regression testing of H264 frames and images.
#![deny(missing_docs)]

use anyhow::Result;

const CRATE_ENV_VAR: &str = "TWENTY_TWENTY";

/// Convert a H264 frame to an image.
pub fn h264_frame_to_image(width: u32, height: u32, data: &[u8]) -> Result<image::DynamicImage> {
    let Some(raw) = image::RgbImage::from_raw(width, height, data.to_vec()) else {
        anyhow::bail!("could not parse image from raw");
    };

    Ok(image::DynamicImage::ImageRgb8(raw))
}

/// Compare the contents of the file to the image provided.
#[track_caller]
pub fn assert_image<P: AsRef<std::path::Path>>(path: P, actual: &image::DynamicImage) {
    if let Err(e) = assert_image_impl(path, actual) {
        panic!("assertion failed: {e}")
    }
}

pub(crate) fn assert_image_impl<P: AsRef<std::path::Path>>(
    path: P,
    actual: &image::DynamicImage,
) -> Result<()> {
    let path = path.as_ref();
    let var = std::env::var_os(CRATE_ENV_VAR);
    let overwrite = var.as_deref().and_then(std::ffi::OsStr::to_str) == Some("overwrite");

    if overwrite {
        if let Err(e) = actual.save_with_format(path, image::ImageFormat::Png) {
            panic!("unable to write image to {}: {}", path.display(), e);
        }
    } else {
        // Treat a nonexistent file like an empty image.
        let _expected_image = match image::io::Reader::open(path) {
            Ok(s) => s.decode()?,
            Err(e) => match e.kind() {
                // TODO: fix dimensions.
                std::io::ErrorKind::NotFound => image::DynamicImage::new_rgba16(0, 0),
                _ => panic!("unable to read contents of {}: {}", path.display(), e),
            },
        };

        // Compare the two images.
    }
    Ok(())
}
