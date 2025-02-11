//! The `twenty-twenty` crate allows for visual regression testing of H.264 frames (enabled with the `h264` feature) as well as images.
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
//! To compare H.264 frames you will need `ffmpeg` installed on your system and the `h264` feature enabled to use this crate, which relies on
//! the [Rust ffmpeg bindings](https://docs.rs/ffmpeg-next/latest/ffmpeg_next/) to convert the H.264 frames to images.
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
//!
//! # Usage in tests
//!
//! 1. Write a test, for example:
//!
//!   ```
//!   // tests/twenty_twenty.rs
//!   #[test]
//!   fn example_test() {
//!       # fn get_image() -> image::DynamicImage {
//!       #    image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap()
//!       # }
//!       let actual = get_image();
//!       twenty_twenty::assert_image("tests/dog1.png", &actual, 0.9);
//!   }
//!   ```
//!
//! 2. Run the test and have it write `actual` to disk next to the file the test is in, in this case `tests/dog1.png`:
//!
//!   ```sh
//!   TWENTY_TWENTY=overwrite cargo test example_test
//!   ```
//!
//! 3. Review the output image and ensure it is a correct reference image.
//!
//! 4. Run `cargo test`. If the generated image changes and differs from the image written to disk, the test will fail.
//!
//! # Storing artifacts in CI
//!
//! Use either `TWENTY_TWENTY=store-artifact` or `TWENTY_TWENTY=store-artifact-on-mismatch` to save artifacts to the `artifacts/` directory. The latter can be used to only store failing tests for review and repair.

#![deny(missing_docs)]

#[cfg(feature = "h264")]
mod h264;
#[cfg(feature = "h264")]
pub use h264::assert_h264_frame;

const CRATE_ENV_VAR: &str = "TWENTY_TWENTY";

/// The different modes available for the TWENTY_TWENTY environment variable.
#[derive(Default, PartialEq)]
enum Mode {
    /// Only assert the image diff is within the given threshold.
    #[default]
    Default,
    /// Overwrite the file we are comparing against, i.e. accept the changes of the diff.
    Overwrite,
    /// Store the files on disk always (for now make all paths relative to `artifacts/`).
    StoreArtifact,
    /// Store the files on disk when they don't match (for now make all paths relative to `artifacts/`).
    StoreArtifactOnMismatch,
}

impl std::str::FromStr for Mode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "overwrite" => Mode::Overwrite,
            "store-artifact" => Mode::StoreArtifact,
            "store-artifact-on-mismatch" => Mode::StoreArtifactOnMismatch,
            _ => Mode::Default,
        })
    }
}

/// Compare the contents of the file to the image provided.
///
/// `min_permissible_similarity` is a floating point value between `0.0` and `1.0`. If the two compared images are less similar than the `min_permissible_similarity` threshold,
/// the test will fail.
///
/// The score is also a floating point value between `0.0` and `1.0`.
/// If the images are identical, the score will be `1.0`.
#[track_caller]
pub fn assert_image<P: AsRef<std::path::Path>>(path: P, actual: &image::DynamicImage, min_permissible_similarity: f64) {
    if let Err(e) = assert_image_impl(path, actual, min_permissible_similarity) {
        panic!("assertion failed: {e}")
    }
}

pub(crate) fn assert_image_impl<P: AsRef<std::path::Path>>(
    path: P,
    actual: &image::DynamicImage,
    min_permissible_similarity: f64,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    let var = std::env::var_os(CRATE_ENV_VAR);
    let mode: Mode = var
        .as_deref()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .parse()
        .unwrap_or_default();

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

#[cfg(test)]
mod tests {
    use super::assert_image;

    #[test]
    fn test_overwrite_mode() {
        std::fs::create_dir_all("tests/tmp").unwrap();
        std::fs::copy("tests/dog1.png", "tests/tmp/initial-grid.png").unwrap();
        let expected_image = image::io::Reader::open("tests/initial-grid.png")
            .unwrap()
            .decode()
            .unwrap();
        std::env::set_var("TWENTY_TWENTY", "overwrite");
        assert_image("tests/tmp/initial-grid.png", &expected_image, 1.0);
        std::env::set_var("TWENTY_TWENTY", "");
        assert_image("tests/tmp/initial-grid.png", &expected_image, 1.0);
    }

    #[test]
    fn test_store_artifact_mode() {
        let expected_image = image::io::Reader::open("tests/initial-grid.png")
            .unwrap()
            .decode()
            .unwrap();
        std::env::set_var("TWENTY_TWENTY", "store-artifact");
        assert_image("tests/initial-grid.png", &expected_image, 1.0);
        std::env::set_var("TWENTY_TWENTY", "");
        assert_image("artifacts/tests/initial-grid.png", &expected_image, 1.0);
    }

    #[test]
    fn test_store_artifact_if_mismatch_mode() {
        let expected_image = image::io::Reader::open("tests/initial-grid.png")
            .unwrap()
            .decode()
            .unwrap();
        std::env::set_var("TWENTY_TWENTY", "store-artifact-on-mismatch");
        // We expect the panic, so we just catch and continue on.
        let _result = std::panic::catch_unwind(|| {
            assert_image("tests/multiple-frames.png", &expected_image, 1.0);
        });
        std::env::set_var("TWENTY_TWENTY", "");
        assert_image("artifacts/tests/multiple-frames.png", &expected_image, 1.0);
    }
}
