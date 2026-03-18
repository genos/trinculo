//! Common utilities.
use image::{GrayImage, ImageFormat};
use std::{
    fs,
    io::{self, Cursor},
    path::Path,
};

/// Errors that can arise from using one of these utility functions.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    File(#[from] io::Error),
    #[error("Image size doesn't fit: {0}")]
    ImageSize(u32),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
}

/// Read the input Prospero program.
///
/// # Errors
/// If something goes wrong in trying to read the file.
pub fn read_prospero() -> Result<String, Error> {
    Ok(fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/prospero.vm"
    ))?)
}

/// Convert the bytes into an image.
///
/// # Errors
/// If the size is too large.
pub fn to_image(image_size: u32, bytes: Vec<u8>) -> Result<GrayImage, Error> {
    GrayImage::from_raw(image_size, image_size, bytes).ok_or(Error::ImageSize(image_size))
}

/// Write the bytes as a gray PNG.
///
/// # Errors
/// If the size is too large, or something else goes wrong in writing the image.
pub fn write_image(image_size: u32, bytes: Vec<u8>, p: impl AsRef<Path>) -> Result<(), Error> {
    to_image(image_size, bytes)?.save(p)?;
    Ok(())
}

/// Convert the image to a PNG's bytes (for testing).
///
/// # Errors
/// If converting to PNG goes awry.
pub fn to_png(image: &GrayImage) -> Result<Vec<u8>, Error> {
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn read_prospero_ok() {
        assert!(read_prospero().is_ok());
    }

    #[test]
    fn to_image_ok() {
        assert!(to_image(8, vec![0u8; 8 * 8]).is_ok());
        assert!(to_image(1024, vec![0u8; 8 * 8]).is_err());
    }

    #[test]
    fn write_image_ok() {
        let f = NamedTempFile::with_suffix(".png");
        assert!(f.is_ok());
        assert!(write_image(8, vec![0u8; 8 * 8], f.unwrap()).is_ok());
        let f = NamedTempFile::with_suffix(".png");
        assert!(f.is_ok());
        assert!(write_image(1024, vec![0u8; 8 * 8], f.unwrap()).is_err());
        let f = NamedTempFile::with_suffix(".definitely_not_a_png");
        assert!(f.is_ok());
        assert!(write_image(8, vec![0u8; 8 * 8], f.unwrap()).is_err());
    }
}
