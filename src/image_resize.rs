use bytes::Bytes;
use fast_image_resize::{IntoImageView, PixelType, ResizeError};
use image::{codecs::webp::WebPEncoder, io::Reader as ImageReader, ImageEncoder, ImageError, ImageFormat};
use thiserror::Error;
use std::io::Cursor;

#[derive(Error, Debug)]
pub enum ImgResizeError {
    #[error("Resize error")]
    ResizeError(#[from]ResizeError),
    #[error("Image bytes write error")]
    ImageWriteError(#[from]ImageError),
    #[error("IO Error")]
    IoError(#[from]std::io::Error),
    #[error("Cannot determine image format")]
    FormatDeterminitionErr,
    #[error("No resize needed")]
    NoResizeNeeded,
}

/// Resize given image to the given size
/// ## Arguments:
/// * `bytes` - bytes of image file
/// * `biggest_size` - size of bounding box the image should be downscaled to
pub fn resize_fast(bytes: &Bytes, biggest_size: u32) -> std::result::Result<Vec<u8>, ImgResizeError> {
    let cursor = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?;

    let Some(format) = cursor.format() else {
        return Err(ImgResizeError::FormatDeterminitionErr);
    };

    let img = cursor.decode()?;

    let need_resizing = img.width() >= biggest_size || img.height() >= biggest_size;
    if !need_resizing {
        if format == ImageFormat::WebP {
            return Err(ImgResizeError::NoResizeNeeded);
        }
        let mut result = Cursor::new(Vec::new());
        img.write_to(&mut result, ImageFormat::WebP)?;
        return Ok(result.into_inner());
    }

    let (width, height) = {
        let ratio = img.width() as f32 / img.height() as f32;
        if ratio < 1.0 {
            ((biggest_size as f32 * ratio) as u32, biggest_size)
        } else {
            (biggest_size, (biggest_size as f32 / ratio) as u32)
        }
    };

    let mut dst_image = fast_image_resize::images::Image::new(
        width,
        height,
        img.pixel_type().unwrap_or(PixelType::U8x3),
    );
    let mut resizer = fast_image_resize::Resizer::new();
    resizer.resize(&img, &mut dst_image, None)?;
    
    let mut result: Vec<u8> = Vec::new();
    WebPEncoder::new_lossless(&mut result)
        .write_image(
            dst_image.buffer(),
            width,
            height,
            img.color().into(),
        )?;

    Ok(result)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_no_need_to_resize() {
        let data = std::fs::read("test_data/img/small.webp").unwrap();
        let bytes = Bytes::from(data);
        let result = resize_fast(&bytes, 400);

        match result {
            Err(ImgResizeError::NoResizeNeeded) => (),
            _ => panic!("Expected: {}", ImgResizeError::NoResizeNeeded),
        }
    }

    #[test]
    fn test_no_resize_just_recompression_to_webp() {
        let data = std::fs::read("test_data/img/small.png").unwrap();
        let bytes = Bytes::from(data);
        let result = resize_fast(&bytes, 400).unwrap();

        let new_format = ImageReader::new(Cursor::new(result))
            .with_guessed_format().unwrap().format().unwrap();

        assert_eq!(new_format, ImageFormat::WebP);
    }
}