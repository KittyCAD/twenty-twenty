# twenty-twenty

The `twenty-twenty` crate allows for visual regression testing of H.264 frames (enabled with the
`h264` feature) as well as images. It makes it easy to update the contents when they should be
updated to match the new results.

Each function takes a minimum permissible similarity, which is the lowest possible "score" you are
willing for the image comparison to return. If the resulting score is less than the minimum, the
test will fail. The score must be a number between 0 and 1. If the images are the exact same, the
score will be 1.

The underlying algorithm is SSIM, which is a perceptual metric that quantifies the image quality
degradation that is caused by processing such as data compression or by losses in data transmission.
More information can be found [here](https://en.wikipedia.org/wiki/Structural_similarity).

To compare H.264 frames you will need `ffmpeg` installed on your system and the `h264` feature
enabled to use this crate, which relies on the
[Rust ffmpeg bindings](https://docs.rs/ffmpeg-next/latest/ffmpeg_next/) to convert the H.264 frames
to images.

Use it like this for an H.264 frame:

```rust
let actual = get_h264_frame();
twenty_twenty::assert_h264_frame("tests/initial-grid.png", &actual, 0.9);
```

Use it like this for an image:

```rust
let actual = get_image();
twenty_twenty::assert_image("tests/dog1.png", &actual, 0.9);
```

If the output doesn't match, the program will `panic!` and emit the difference in the score.

To accept the changes from `get_h264_frame()` or `get_image()`, run with `TWENTY_TWENTY=overwrite`.

## Usage in tests

1. Write a test, for example:

```
// tests/twenty_twenty.rs
#[test]
fn example_test() {
    # fn get_image() -> image::DynamicImage {
    #    image::io::Reader::open("tests/dog1.png").unwrap().decode().unwrap()
    # }
    let actual = get_image();
    twenty_twenty::assert_image("tests/dog1.png", &actual, 0.9);
}
```

2. Run the test and have it write `actual` to disk next to the file the test is in, in this case
   `tests/dog1.png`:

```sh
TWENTY_TWENTY=overwrite cargo test example_test
```

3. Review the output image and ensure it is a correct reference image.

4. Run `cargo test`. If the generated image changes and differs from the image written to disk, the
   test will fail.

## Storing artifacts in CI

Use either `TWENTY_TWENTY=store-artifact` or `TWENTY_TWENTY=store-artifact-on-mismatch` to save
artifacts to the `artifacts/` directory. The latter can be used to only store failing tests for
review and repair.

## Publishing a new release

We have a GitHub action that pushes our releases
[here](https://github.com/KittyCAD/twenty-twenty/blob/main/.github/workflows/make-release.yml). It
is triggered by pushing a new tag. So do the following:

1. Bump the version in `Cargo.toml`. Commit it and push it up to the repo.
2. Create a tag with the new version: `git tag -sa v$(VERSION) -m "v$(VERSION)"`
3. Push the tag to the repo: `git push origin v$(VERSION)`
