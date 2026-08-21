//! The `twenty-twenty` crate allows for visual regression testing of H.264 frames (enabled with the
//! `h264` feature) as well as images. It makes it easy to update the contents when they should be
//! updated to match the new results.
//!
//! Each function takes a minimum permissible similarity, which is the lowest possible "score" you
//! are willing for the image comparison to return. If the resulting score is less than the minimum,
//! the test will fail. The score must be a number between 0 and 1. If the images are the exact
//! same, the score will be 1.
//!
//! The underlying algorithm is SSIM, which is a perceptual metric that quantifies the image quality
//! degradation that is caused by processing such as data compression or by losses in data
//! transmission. More information can be found
//! [here](https://en.wikipedia.org/wiki/Structural_similarity).
//!
//! To compare H.264 frames you will need `ffmpeg` installed on your system and the `h264` feature
//! enabled to use this crate, which relies on the [Rust ffmpeg
//! bindings](https://docs.rs/ffmpeg-next/latest/ffmpeg_next/) to convert the H.264 frames to
//! images.
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
//! If the output doesn't match, [`assert_image`] will `panic!` and emit the difference in the score.
//! Use [`try_assert_image`] to handle the error instead.
//!
//! To accept the changes from `get_h264_frame()` or `get_image()`, run with
//! `TWENTY_TWENTY=overwrite`.
//!
//! # Usage in tests
//!
//! 1. Write a test, for example:
//!
//!   ```no_run
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
//! 2. Run the test and have it write `actual` to disk next to the file the test is in, in this case
//!    `tests/dog1.png`:
//!
//!   ```sh
//!   TWENTY_TWENTY=overwrite cargo test example_test
//!   ```
//!
//! 3. Review the output image and ensure it is a correct reference image.
//!
//! 4. Run `cargo test`. If the generated image changes and differs from the image written to disk,
//!    the test will fail.
//!
//! # Storing artifacts in CI
//!
//! Use either `TWENTY_TWENTY=store-artifact` or `TWENTY_TWENTY=store-artifact-on-mismatch` to save
//! artifacts to the `artifacts/` directory. The latter can be used to only store failing tests for
//! review and repair.

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
    /// Overwrite the file if they don't match, but leave it alone if they do.
    UpdateOnMismatch,
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
            "update" => Mode::UpdateOnMismatch,
            "store-artifact" => Mode::StoreArtifact,
            "store-artifact-on-mismatch" => Mode::StoreArtifactOnMismatch,
            _ => Mode::Default,
        })
    }
}

/// An error returned when an image assertion cannot be completed successfully.
#[derive(Debug)]
#[non_exhaustive]
pub enum AssertImageError {
    /// The expected image could not be read.
    #[non_exhaustive]
    ReadImage {
        /// The path to the expected image.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The expected image could not be decoded.
    #[non_exhaustive]
    DecodeImage {
        /// The path to the expected image.
        path: std::path::PathBuf,
        /// The underlying image error.
        source: image::ImageError,
    },
    /// The expected and actual images could not be compared.
    #[non_exhaustive]
    CompareImages {
        /// The underlying comparison error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A directory needed for an output image could not be created.
    #[non_exhaustive]
    CreateDirectory {
        /// The directory that could not be created.
        path: std::path::PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// An output image could not be written.
    #[non_exhaustive]
    WriteImage {
        /// The path to the output image.
        path: std::path::PathBuf,
        /// The underlying image error.
        source: image::ImageError,
    },
    /// The image similarity score was below the required threshold.
    #[non_exhaustive]
    ImageMismatch {
        /// The path to the expected image.
        path: std::path::PathBuf,
        /// The measured image similarity score.
        score: f64,
        /// The minimum permissible image similarity score.
        min_permissible_similarity: f64,
    },
}

impl std::fmt::Display for AssertImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadImage { path, source } => {
                write!(f, "unable to read contents of {}: {source}", path.display())
            }
            Self::DecodeImage { source, .. } => write!(f, "decoding image from path failed: {source:?}"),
            Self::CompareImages { source } => write!(f, "could not compare the images {source}"),
            Self::CreateDirectory { path, source } => {
                write!(f, "unable to create directory {}: {source}", path.display())
            }
            Self::WriteImage { path, source } => {
                write!(f, "unable to write image to {}: {source}", path.display())
            }
            Self::ImageMismatch {
                path,
                score,
                min_permissible_similarity,
            } => write!(
                f,
                r#"image (`{}`) score is `{}` which is less than min_permissible_similarity `{}`
                set {}=overwrite if these changes are intentional"#,
                path.display(),
                score,
                min_permissible_similarity,
                CRATE_ENV_VAR
            ),
        }
    }
}

impl std::error::Error for AssertImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadImage { source, .. } | Self::CreateDirectory { source, .. } => Some(source),
            Self::DecodeImage { source, .. } | Self::WriteImage { source, .. } => Some(source),
            Self::CompareImages { source } => Some(source.as_ref()),
            Self::ImageMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Copy)]
enum OperationalFailure {
    Panic,
    Return,
}

impl OperationalFailure {
    #[track_caller]
    fn handle<T>(self, error: AssertImageError) -> Result<T, AssertImageError> {
        match self {
            Self::Panic => panic!("{error}"),
            Self::Return => Err(error),
        }
    }

    #[track_caller]
    fn handle_with_panic_message<T>(
        self,
        error: AssertImageError,
        panic_message: String,
    ) -> Result<T, AssertImageError> {
        match self {
            Self::Panic => panic!("{panic_message}"),
            Self::Return => Err(error),
        }
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
    if let Err(e) = assert_image_impl(path, actual, min_permissible_similarity, OperationalFailure::Panic) {
        panic!("assertion failed: {e}")
    }
}

/// Compare the contents of the file to the image provided, returning an error if they do not match.
///
/// This is the non-panicking equivalent of [`assert_image`].
/// `min_permissible_similarity` and the `TWENTY_TWENTY` modes behave identically for both functions.
///
/// # Errors
///
/// Returns an error if the images do not meet the similarity threshold or if reading, decoding,
/// comparing, or writing an image fails.
pub fn try_assert_image<P: AsRef<std::path::Path>>(
    path: P,
    actual: &image::DynamicImage,
    min_permissible_similarity: f64,
) -> Result<(), AssertImageError> {
    assert_image_impl(path, actual, min_permissible_similarity, OperationalFailure::Return)
}

fn assert_image_impl<P: AsRef<std::path::Path>>(
    path: P,
    actual: &image::DynamicImage,
    min_permissible_similarity: f64,
    operational_failure: OperationalFailure,
) -> Result<(), AssertImageError> {
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
            operational_failure.handle(AssertImageError::WriteImage {
                path: path.to_path_buf(),
                source: e,
            })?;
        }
        return Ok(());
    }

    // Treat a nonexistent file like an empty image.
    let expected = match image::io::Reader::open(path) {
        Ok(s) => match s.decode() {
            Ok(image) => image,
            Err(e) => operational_failure.handle(AssertImageError::DecodeImage {
                path: path.to_path_buf(),
                source: e,
            })?,
        },
        Err(e) => match e.kind() {
            // We take the dimensions from the original image.
            std::io::ErrorKind::NotFound => image::DynamicImage::new_rgba16(actual.width(), actual.height()),
            _ => operational_failure.handle(AssertImageError::ReadImage {
                path: path.to_path_buf(),
                source: e,
            })?,
        },
    };

    // Compare the two images.
    let result = match image_compare::rgba_hybrid_compare(&expected.to_rgba8(), &actual.to_rgba8()) {
        Ok(result) => result,
        Err(source) => operational_failure.handle(AssertImageError::CompareImages {
            source: Box::new(source),
        })?,
    };

    // The SSIM score should be near 0, this is tweakable from the consumer, since they likely
    // have different thresholds.
    let image_mismatch = result.score < min_permissible_similarity;

    if mode == Mode::StoreArtifact || (mode == Mode::StoreArtifactOnMismatch && image_mismatch) {
        let artifact_path = std::path::Path::new("artifacts/").join(path);
        if let Some(parent) = artifact_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                operational_failure.handle(AssertImageError::CreateDirectory {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
        }
        if let Err(source) = actual.save_with_format(&artifact_path, image::ImageFormat::Png) {
            let panic_message = format!("unable to write image to {}: {source}", path.display());
            operational_failure.handle_with_panic_message(
                AssertImageError::WriteImage {
                    path: artifact_path,
                    source,
                },
                panic_message,
            )?;
        }
    }

    if image_mismatch {
        if mode == Mode::UpdateOnMismatch {
            if let Err(e) = actual.save_with_format(path, image::ImageFormat::Png) {
                operational_failure.handle(AssertImageError::WriteImage {
                    path: path.to_path_buf(),
                    source: e,
                })?;
            }
            return Ok(());
        }
        return Err(AssertImageError::ImageMismatch {
            path: path.to_path_buf(),
            score: result.score,
            min_permissible_similarity,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{assert_image, try_assert_image, AssertImageError};

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_overwrite_mode() {
        let _guard = TEST_LOCK.lock().unwrap();
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
        let _guard = TEST_LOCK.lock().unwrap();
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
        let _guard = TEST_LOCK.lock().unwrap();
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

    #[test]
    fn matching_image_returns_ok() {
        let _guard = TEST_LOCK.lock().unwrap();
        let actual = image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap();

        try_assert_image("tests/dog1.png", &actual, 1.0).unwrap();
    }

    #[test]
    fn image_mismatch_returns_error() {
        let _guard = TEST_LOCK.lock().unwrap();
        let mut actual = image::io::Reader::open("tests/dog1.png")
            .unwrap()
            .decode()
            .unwrap()
            .to_rgba8();
        for pixel in actual.pixels_mut() {
            pixel.0[0] ^= u8::MAX;
        }
        let actual = image::DynamicImage::ImageRgba8(actual);

        let error = try_assert_image("tests/dog1.png", &actual, 1.0).unwrap_err();

        assert!(matches!(&error, AssertImageError::ImageMismatch { .. }));
        assert!(error.to_string().starts_with("image (`tests/dog1.png`) score is `"));
    }

    #[test]
    fn malformed_image_returns_error() {
        let _guard = TEST_LOCK.lock().unwrap();
        let invalid_image = std::env::temp_dir().join(format!("twenty-twenty-invalid-{}.png", std::process::id()));
        std::fs::write(&invalid_image, b"not a PNG").unwrap();
        let actual = image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap();

        let error = try_assert_image(&invalid_image, &actual, 1.0).unwrap_err();
        let panic = std::panic::catch_unwind(|| assert_image(&invalid_image, &actual, 1.0)).unwrap_err();
        let panic_message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap();

        std::fs::remove_file(invalid_image).unwrap();
        assert!(matches!(&error, AssertImageError::DecodeImage { .. }));
        assert!(error.to_string().starts_with("decoding image from path failed:"));
        assert!(panic_message.starts_with("decoding image from path failed:"));
    }

    #[test]
    fn artifact_write_error_reports_output_path_and_preserves_panic() {
        let _guard = TEST_LOCK.lock().unwrap();
        let expected_path =
            std::path::PathBuf::from(format!("tests/try-assert-image-artifact-{}.png", std::process::id()));
        let artifact_path = std::path::Path::new("artifacts").join(&expected_path);
        std::fs::copy("tests/dog1.png", &expected_path).unwrap();
        std::fs::create_dir_all(&artifact_path).unwrap();
        let actual = image::io::Reader::open(&expected_path).unwrap().decode().unwrap();

        std::env::set_var("TWENTY_TWENTY", "store-artifact");
        let error = try_assert_image(&expected_path, &actual, 1.0);
        std::env::remove_var("TWENTY_TWENTY");
        let error = error.unwrap_err();

        std::env::set_var("TWENTY_TWENTY", "store-artifact");
        let panic = std::panic::catch_unwind(|| assert_image(&expected_path, &actual, 1.0));
        std::env::remove_var("TWENTY_TWENTY");
        let panic = panic.unwrap_err();
        let panic_message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap();

        std::fs::remove_file(&expected_path).unwrap();
        std::fs::remove_dir_all(&artifact_path).unwrap();
        match error {
            AssertImageError::WriteImage { path, .. } => assert_eq!(path, artifact_path),
            error => panic!("expected a write error, got {error:?}"),
        }
        assert!(panic_message.starts_with(&format!("unable to write image to {}:", expected_path.display())));
    }
}
