//! The `twenty-twenty` library allows for visual regression testing of H.264 frames and images.
//! It makes it easy to update the contents when should be updated to match the new results.
//!
//! Each function takes a score threshold, which is the lowest possible "score" you are willing for
//! the image comparison to return. If the resulting score is less than than the threshold, the test
//! will fail. The score must be a number between 0 and 1. If the images are the exact same, the
//! score will be 1.
//!
//! Use it like this for an H.264 frame:
//!
//! ```no_run
//! # fn get_frame() -> (u32, u32, Vec<u8>) {
//! #     (0, 0, vec![])
//! # }
//!
//! let (width, height, actual) = get_frame();
//! twenty_twenty::assert_h264_frame("frame_image.png", width, height, &actual, 0.9);
//! ```
//! Use it like this for an image:
//!
//! ```no_run
//! # fn get_image() -> image::DynamicImage {
//! #    todo!()
//! # }
//!
//! let actual = get_image();
//! twenty_twenty::assert_image("og_image.png", &actual, 0.9);
//! ```
//!
//! If the output doesn't match, the program will `panic!` and emit the
//! difference in the score.
//!
//! To accept the changes from `get_frame()` or `get_image()`, run with `TWENTY_TWENTY=overwrite`.

#![deny(missing_docs)]

use anyhow::Result;

const CRATE_ENV_VAR: &str = "TWENTY_TWENTY";

/// Compare the contents of the file to the image provided.
/// The threshold is the lowest possible "score" you are willing for the image
/// comparison to return. If the resulting score is less than than the threshold,
/// the test will fail.
/// The score is a float between 0 and 1.
/// If the images are the exact same, the score will be 1.
#[track_caller]
pub fn assert_image<P: AsRef<std::path::Path>>(
    path: P,
    actual: &image::DynamicImage,
    threshold: f64,
) {
    if let Err(e) = assert_image_impl(path, actual, threshold) {
        panic!("assertion failed: {e}")
    }
}

/// Compare the contents of the file to the H.264 frame provided.
/// The threshold is the lowest possible "score" you are willing for the image
/// comparison to return. If the resulting score is less than than the threshold,
/// the test will fail.
/// The score is a float between 0 and 1.
/// If the images are the exact same, the score will be 1.
#[track_caller]
pub fn assert_h264_frame<P: AsRef<std::path::Path>>(
    path: P,
    width: u32,
    height: u32,
    actual: &[u8],
    threshold: f64,
) {
    match h264_frame_to_image(width, height, actual) {
        Ok(image) => {
            if let Err(e) = assert_image_impl(path, &image, threshold) {
                panic!("assertion failed: {e}")
            }
        }
        Err(e) => {
            panic!("could not convert H.264 frame to image: {e}")
        }
    }
}

// Convert a H264 frame to an image.
pub(crate) fn h264_frame_to_image(
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<image::DynamicImage> {
    let Some(raw) = image::RgbaImage::from_raw(width, height, data.to_vec()) else {
        anyhow::bail!("could not parse image from raw");
    };

    Ok(image::DynamicImage::ImageRgba8(raw))
}

pub(crate) fn assert_image_impl<P: AsRef<std::path::Path>>(
    path: P,
    actual: &image::DynamicImage,
    threshold: f64,
) -> Result<(), String> {
    let path = path.as_ref();
    let var = std::env::var_os(CRATE_ENV_VAR);
    let overwrite = var.as_deref().and_then(std::ffi::OsStr::to_str) == Some("overwrite");

    if overwrite {
        if let Err(e) = actual.save_with_format(path, image::ImageFormat::Png) {
            panic!("unable to write image to {}: {}", path.display(), e);
        }
    } else {
        // Treat a nonexistent file like an empty image.
        let expected = match image::io::Reader::open(path) {
            Ok(s) => s.decode().expect("decoding image from path failed"),
            Err(e) => match e.kind() {
                // TODO: fix dimensions.
                std::io::ErrorKind::NotFound => image::DynamicImage::new_rgba16(0, 0),
                _ => panic!("unable to read contents of {}: {}", path.display(), e),
            },
        };

        // Compare the two images.
        let result =
            match image_compare::rgba_hybrid_compare(&expected.to_rgba8(), &actual.to_rgba8()) {
                Ok(result) => result,
                Err(err) => {
                    panic!("could not compare the images {err}")
                }
            };

        // The SSIM score should be near 0, this is tweakable from the consumer, since they likely
        // have different thresholds.
        if result.score < threshold {
            return Err(format!(
                r#"image (`{}`) score is `{}` which is less than than threshold `{}`
                set {}=overwrite if these changes are intentional"#,
                path.display(),
                result.score,
                threshold,
                CRATE_ENV_VAR
            ));
        }
    }

    Ok(())
}
