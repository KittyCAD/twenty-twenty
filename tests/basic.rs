use twenty_twenty::assert_image;

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
