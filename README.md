# twenty-twenty

The `twenty-twenty` library allows for visual regression testing of H.264 frames and images.
It makes it easy to update the contents when they should be updated to match the new results.

Each function takes a score threshold, which is the lowest possible "score" you are willing for
the image comparison to return. If the resulting score is less than the threshold, the test
will fail. The score must be a number between 0 and 1. If the images are the exact same, the
score will be 1.

The underlying algorithm is SSIM, which is a perceptual metric that quantifies the image
quality degradation that is caused by processing such as data compression or by losses in data
transmission. More information can be found [here](https://en.wikipedia.org/wiki/Structural_similarity).

You will need `ffmpeg` installed on your system to use this library. This library uses
the [ffmpeg bindings](https://docs.rs/ffmpeg-next/latest/ffmpeg_next/) in rust to convert the H.264 frames to images.

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

If the output doesn't match, the program will `panic!` and emit the
difference in the score.

To accept the changes from `get_h264_frame()` or `get_image()`, run with `TWENTY_TWENTY=overwrite`.

## Publishing a new release

We have a GitHub action that pushes our releases [here](https://github.com/KittyCAD/twenty-twenty/blob/main/.github/workflows/make-release.yml). It is triggered by
pushing a new tag. So do the following:

1. Bump the version in `Cargo.toml`. Commit it and push it up to the repo.
2. Create a tag with the new version: `git tag -sa v$(VERSION) -m "v$(VERSION)"`
3. Push the tag to the repo: `git push origin v$(VERSION)`
