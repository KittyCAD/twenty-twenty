use twenty_twenty::{assert_image, try_assert_image};

#[test]
fn matching_image_returns_ok() {
    let actual = image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap();

    try_assert_image("tests/dog1.png", &actual, 1.0).unwrap();
}

#[test]
fn image_mismatch_returns_error() {
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

    assert!(error.to_string().starts_with("image (`tests/dog1.png`) score is `"));
}

#[test]
fn malformed_image_returns_error() {
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
    assert!(error.to_string().starts_with("decoding image from path failed:"));
    assert!(panic_message.starts_with("decoding image from path failed:"));
}
