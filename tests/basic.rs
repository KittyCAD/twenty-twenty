use twenty_twenty::{assert_h264_frame, assert_image};

#[test]
fn good() {
    let actual = image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap();
    assert_image("tests/dog1.png", &actual, 1.0);
}

#[test]
#[should_panic]
fn bad() {
    let actual = image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap();
    assert_image("tests/dog2.png", &actual, 1.0);
}

#[test]
fn good_h264() {
    let actual = std::fs::read("tests/initial-grid.h264").unwrap();
    assert_h264_frame("tests/initial-grid.png", &actual, 0.99);
}

#[test]
fn good_h264_multiple_frames() {
    let actual = std::fs::read("tests/multiple-frames.h264").unwrap();
    assert_h264_frame("tests/multiple-frames.png", &actual, 0.99);
}
