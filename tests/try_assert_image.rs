use twenty_twenty::{assert_image, try_assert_image, AssertImageError};

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let expected_path = std::path::PathBuf::from(format!("tests/try-assert-image-artifact-{}.png", std::process::id()));
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
