use std::io::Write;

use anyhow::Result;
use ffmpeg_next as ffmpeg;

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
            if let Err(e) = super::assert_image_impl(path, &image, min_permissible_similarity) {
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
    let Some(raw) = image::RgbImage::from_raw(video_frame.width(), video_frame.height(), video_frame.data(0).to_vec())
    else {
        anyhow::bail!("the container was not big enough as per: https://docs.rs/image/latest/image/struct.ImageBuffer.html#method.from_raw");
    };

    Ok(image::DynamicImage::ImageRgb8(raw))
}
