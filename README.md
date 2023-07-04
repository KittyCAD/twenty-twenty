# twenty-twenty

The `twenty-twenty` library allows for visual regression testing of H.264 frames and images.
It makes it easy to update the contents when should be updated to match the new results.

Each function takes a score threshold, which is the lowest possible "score" you are willing for
the image comparison to return. If the resulting score is less than than the threshold, the test
will fail. The score must be a number between 0 and 1. If the images are the exact same, the
score will be 1.

Use it like this for an H.264 frame:

```rust

let (width, height, actual) = get_frame();
twenty_twenty::assert_h264_frame("frame_image.png", width, height, &actual, 0.9);
```
Use it like this for an image:

```rust

let actual = get_image();
twenty_twenty::assert_image("og_image.png", &actual, 0.9);
```

If the output doesn't match, the program will `panic!` and emit the
difference in the score.

To accept the changes from `get_frame()` or `get_image()`, run with `TWENTY_TWENTY=overwrite`.
