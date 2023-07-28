//! The `twenty-twenty` library allows for visual regression testing of H.264 frames and images.
//! It makes it easy to update the contents when they should be updated to match the new results.
//!
//! Each function takes a minimum permissible similarity, which is the lowest possible "score" you are willing for
//! the image comparison to return. If the resulting score is less than the minimum, the test
//! will fail. The score must be a number between 0 and 1. If the images are the exact same, the
//! score will be 1.
//!
//! The underlying algorithm is SSIM, which is a perceptual metric that quantifies the image
//! quality degradation that is caused by processing such as data compression or by losses in data
//! transmission. More information can be found [here](https://en.wikipedia.org/wiki/Structural_similarity).
//!
//! You will need `ffmpeg` installed on your system to use this library. This library uses
//! the [ffmpeg bindings](https://docs.rs/ffmpeg-next/latest/ffmpeg_next/) in rust to convert the H.264 frames to images.
//!
//! Use it like this for an H.264 frame:
//!
//! ```rust
//! # fn get_h264_frame() -> Vec<u8> {
//! #     std::fs::read("tests/initial-grid.h264").unwrap()
//! # }
//! let actual = get_h264_frame();
//! twenty_twenty::assert_h264_frame("tests/initial-grid.png", &actual, 0.9);
//! ```
//! Use it like this for an image:
//!
//! ```rust
//! # fn get_image() -> image::DynamicImage {
//! #    image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap()
//! # }
//! let actual = get_image();
//! twenty_twenty::assert_image("tests/dog1.png", &actual, 0.9);
//! ```
//!
//! If the output doesn't match, the program will `panic!` and emit the
//! difference in the score.
//!
//! To accept the changes from `get_h264_frame()` or `get_image()`, run with `TWENTY_TWENTY=overwrite`.

#![deny(missing_docs)]

use std::io::Write;

use anyhow::Result;
use ffmpeg_next as ffmpeg;

const CRATE_ENV_VAR: &str = "TWENTY_TWENTY";

/// The different modes available for the TWENTY_TWENTY environment variable.
#[derive(PartialEq)]
enum Mode {
    /// Overwrite the file we are comparing against, i.e. accept the changes of the diff.
    Overwrite,
    /// Store the files on disk when they don't match (for now make all paths relative to `artifacts/`).
    StoreArtifactOnMismatch,
    /// Store the files on disk always (for now make all paths relative to `artifacts/`).
    StoreArtifact,
    /// Only assert the image diff is within the given threshold.
    Default,
}

/// Compare the contents of the file to the image provided.
/// If the two are less similar than the `min_permissible_similarity` threshold,
/// the test will fail.
/// The `min_permissible_similarity` is a float between 0 and 1.
/// The score is a float between 0 and 1.
/// If the images are the exact same, the score will be 1.
#[track_caller]
pub fn assert_image<P: AsRef<std::path::Path>>(path: P, actual: &image::DynamicImage, min_permissible_similarity: f64) {
    if let Err(e) = assert_image_impl(path, actual, min_permissible_similarity) {
        panic!("assertion failed: {e}")
    }
}

/// Compare the contents of the file to the H.264 frame provided.
/// If the two are less similar than the `min_permissible_similarity` threshold,
/// the test will fail.
/// The `min_permissible_similarity` is a float between 0 and 1.
/// If the images are the exact same, the score will be 1.
/// This compares the H.264 frame to a PNG. This is because then the diff will be easily visible
/// in a UI like GitHub's.
#[track_caller]
pub fn assert_h264_frame<P: AsRef<std::path::Path>>(path: P, actual: &[u8], min_permissible_similarity: f64) {
    match h264_frame_to_image(actual) {
        Ok(image) => {
            if let Err(e) = assert_image_impl(path, &image, min_permissible_similarity) {
                panic!("assertion failed: {e}")
            }
        }
        Err(e) => {
            panic!("could not convert H.264 frame to image: {e}")
        }
    }
}

// Convert a H264 frame to an image.
pub(crate) fn h264_frame_to_image(data: &[u8]) -> Result<image::DynamicImage> {
    // Initialize the FFmpeg library
    ffmpeg::init()?;

    // Save the frame to a temporary file, we can read back out of.
    // This will automatically be deleted when the program exits.
    // TODO: this sucks we have to write this back out to disk, we should find a better way
    // to create a decoder from just bytes.
    let temp_file_name = std::env::temp_dir().join(format!("{}.h264", uuid::Uuid::new_v4()));
    let mut temp_file = std::fs::File::create(&temp_file_name)?;
    temp_file.write_all(data)?;

    // Create a decoder for the H.264 format
    let ictx = ffmpeg::format::input(&temp_file_name).map_err(|e| anyhow::anyhow!(e))?;
    let input = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let context = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
    let mut video_decoder = context.decoder().video()?;

    // Read the H.264 frame
    let mut video_frame = ffmpeg::frame::Video::empty();
    let packet = ffmpeg::packet::Packet::copy(data);

    // Decode the H.264 frame
    video_decoder.send_packet(&packet)?;
    video_decoder.receive_frame(&mut video_frame)?;
    video_decoder.flush();

    // Get the pixel format of the decoded frame
    let pixel_format = video_frame.format();
    if pixel_format != ffmpeg::format::Pixel::RGB24 {
        let mut converted_video = ffmpeg::frame::Video::empty();
        // Convert the decoded frame to an RGB format.
        video_frame
            .converter(ffmpeg::format::Pixel::RGB24)?
            .run(&video_frame, &mut converted_video)?;
        video_frame = converted_video;
    }

    // Convert the decoded frame to an RGB format
    video_frame.set_format(ffmpeg::format::Pixel::RGB24);

    // Create an image from the RGB frame
    let Some(raw) = image::RgbImage::from_raw(video_frame.width(), video_frame.height(), video_frame.data(0).to_vec()) else {
        anyhow::bail!("the container was not big enough as per: https://docs.rs/image/latest/image/struct.ImageBuffer.html#method.from_raw");
    };

    Ok(image::DynamicImage::ImageRgb8(raw))
}

pub(crate) fn assert_image_impl<P: AsRef<std::path::Path>>(
    path: P,
    actual: &image::DynamicImage,
    min_permissible_similarity: f64,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    let var = std::env::var_os(CRATE_ENV_VAR);
    let mode = match var.as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("overwrite") => Mode::Overwrite,
        Some("store-artifact") => Mode::StoreArtifact,
        Some("store-artifact-on-mismatch") => Mode::StoreArtifactOnMismatch,
        _ => Mode::Default,
    };

    if mode == Mode::Overwrite {
        if let Err(e) = actual.save_with_format(path, image::ImageFormat::Png) {
            panic!("unable to write image to {}: {}", path.display(), e);
        }
        return Ok(());
    }

    // Treat a nonexistent file like an empty image.
    let expected = match image::io::Reader::open(path) {
        Ok(s) => s.decode().expect("decoding image from path failed"),
        Err(e) => match e.kind() {
            // We take the dimensions from the original image.
            std::io::ErrorKind::NotFound => image::DynamicImage::new_rgba16(actual.width(), actual.height()),
            _ => panic!("unable to read contents of {}: {}", path.display(), e),
        },
    };

    // Compare the two images.
    let result = match image_compare::rgba_hybrid_compare(&expected.to_rgba8(), &actual.to_rgba8()) {
        Ok(result) => result,
        Err(err) => {
            panic!("could not compare the images {err}")
        }
    };

    // The SSIM score should be near 0, this is tweakable from the consumer, since they likely
    // have different thresholds.
    let image_mismatch = result.score < min_permissible_similarity;

    if mode == Mode::StoreArtifact || (mode == Mode::StoreArtifactOnMismatch && image_mismatch) {
        let artifact_path = std::path::Path::new("artifacts/").join(path);
        if let Some(parent) = artifact_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Err(e) = actual.save_with_format(artifact_path, image::ImageFormat::Png) {
            panic!("unable to write image to {}: {}", path.display(), e);
        }
    }

    if image_mismatch {
        anyhow::bail!(
            r#"image (`{}`) score is `{}` which is less than min_permissible_similarity `{}`
                set {}=overwrite if these changes are intentional"#,
            path.display(),
            result.score,
            min_permissible_similarity,
            CRATE_ENV_VAR
        )
    }

    Ok(())
}
