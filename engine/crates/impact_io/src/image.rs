//! Image loading and saving.

use anyhow::{Context, Result, bail};
use memmap2::Mmap;
use std::{
    fs::{self, File},
    io::BufWriter,
    path::Path,
};

/// Represents a decoded image with pixel data.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Image {
    /// Metadata for the image.
    pub meta: ImageMetadata,
    /// Raw pixel data.
    pub data: Vec<u8>,
}

/// Metadata for an image.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMetadata {
    /// Width of the image in pixels.
    pub width: u32,
    /// Height of the image in pixels.
    pub height: u32,
    /// Format of the pixel data.
    pub pixel_format: PixelFormat,
}

/// Supported image formats for pixel data.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// RGBA format with 8 bits per channel.
    Rgba8,
    /// Grayscale format with 8 bits per pixel.
    Luma8,
}

const PNG_MAGIC_BYTES: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

const JPEG_MAGIC_BYTES: &[u8; 3] = b"\xff\xd8\xff";

impl Image {
    /// Returns the dimensions of the image as (width, height).
    pub fn dimensions(&self) -> (u32, u32) {
        (self.meta.width, self.meta.height)
    }

    /// Converts the image to RGBA8 format.
    ///
    /// If the image is already in RGBA8 format, returns the data as-is.
    /// If the image is in Luma8 format, converts grayscale to RGBA.
    pub fn into_rgba8(self) -> Vec<u8> {
        match self.meta.pixel_format {
            PixelFormat::Rgba8 => self.data,
            PixelFormat::Luma8 => convert_luma_data_to_rgba(&self.data),
        }
    }

    /// Converts the image to Luma8 (grayscale) format.
    ///
    /// If the image is already in Luma8 format, returns the data as-is.
    /// If the image is in RGBA8 format, converts using luminance formula.
    pub fn into_luma8(self) -> Vec<u8> {
        match self.meta.pixel_format {
            PixelFormat::Luma8 => self.data,
            PixelFormat::Rgba8 => convert_rgba_data_to_luma(&self.data),
        }
    }

    /// Returns true if the image has color information (RGBA8), false if
    /// grayscale (Luma8).
    pub fn has_color(&self) -> bool {
        matches!(self.meta.pixel_format, PixelFormat::Rgba8)
    }
}

/// Reads the metadata of the image at the specified path.
///
/// Supports PNG and JPEG formats (when respective features are enabled).
pub fn read_metadata_for_image_at_path(image_path: impl AsRef<Path>) -> Result<ImageMetadata> {
    let image_path = image_path.as_ref();

    impact_log::debug!("Reading metadata for image at {}", image_path.display());

    let file = File::open(image_path)
        .with_context(|| format!("Failed to open image at {}", image_path.display()))?;

    let mmap = unsafe { Mmap::map(&file) }.with_context(|| {
        format!(
            "Failed to create memory map for image at {}",
            image_path.display()
        )
    })?;

    read_image_metadata_from_bytes(&mmap)
        .with_context(|| format!("Failed to decode image at {}", image_path.display()))
}

/// Loads an image from the specified file path.
///
/// Supports PNG and JPEG formats (when respective features are enabled).
pub fn load_image_from_path(image_path: impl AsRef<Path>) -> Result<Image> {
    let image_path = image_path.as_ref();

    impact_log::debug!("Loading image from {}", image_path.display());

    let buffer = fs::read(image_path)
        .with_context(|| format!("Failed to read image at {}", image_path.display()))?;

    load_image_from_bytes(&buffer)
        .with_context(|| format!("Failed to decode image at {}", image_path.display()))
}

/// Reads the metadata of the image in a byte buffer.
///
/// Supports PNG and JPEG formats (when respective features are enabled).
pub fn read_image_metadata_from_bytes(bytes: &[u8]) -> Result<ImageMetadata> {
    // Detect format based on magic bytes
    if bytes.starts_with(PNG_MAGIC_BYTES) {
        #[cfg(feature = "png")]
        return read_png_metadata_from_reader(bytes);

        #[cfg(not(feature = "png"))]
        bail!("enable the `png` feature to load PNG images");
    } else if bytes.starts_with(JPEG_MAGIC_BYTES) {
        #[cfg(feature = "jpeg")]
        return read_jpeg_metadata_from_bytes(bytes);

        #[cfg(not(feature = "jpeg"))]
        bail!("enable the `jpeg` feature to load JPEG images");
    } else {
        bail!("Unsupported image format or corrupted image data");
    }
}

/// Loads an image from a byte buffer.
///
/// Supports PNG and JPEG formats (when respective features are enabled).
pub fn load_image_from_bytes(bytes: &[u8]) -> Result<Image> {
    // Detect format based on magic bytes
    if bytes.starts_with(PNG_MAGIC_BYTES) {
        #[cfg(feature = "png")]
        return load_png_from_reader(bytes);

        #[cfg(not(feature = "png"))]
        bail!("enable the `png` feature to load PNG images");
    } else if bytes.starts_with(JPEG_MAGIC_BYTES) {
        #[cfg(feature = "jpeg")]
        return load_jpeg_from_bytes(bytes);

        #[cfg(not(feature = "jpeg"))]
        bail!("enable the `jpeg` feature to load JPEG images");
    } else {
        bail!("Unsupported image format or corrupted image data");
    }
}

/// Reads the metadata of a PNG image from a reader.
#[cfg(feature = "png")]
fn read_png_metadata_from_reader(reader: impl std::io::Read) -> Result<ImageMetadata> {
    let decoder = png::Decoder::new(reader);
    let reader = decoder.read_info().context("Failed to read PNG info")?;
    let info = reader.info();

    let pixel_format = match info.color_type {
        // Alpha channel is added if missing when image data is read
        png::ColorType::Rgb | png::ColorType::Rgba => PixelFormat::Rgba8,
        // Alpha channel is stripped if present when image data is read
        png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha => PixelFormat::Luma8,
        png::ColorType::Indexed => bail!("Unsupported PNG color type: {:?}", info.color_type),
    };

    Ok(ImageMetadata {
        width: info.width,
        height: info.height,
        pixel_format,
    })
}

/// Loads a PNG image from a reader.
#[cfg(feature = "png")]
fn load_png_from_reader(reader: impl std::io::Read) -> Result<Image> {
    let decoder = png::Decoder::new(reader);
    let mut reader = decoder.read_info().context("Failed to read PNG info")?;

    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .context("Failed to read PNG frame")?;

    let (width, height) = (info.width, info.height);

    let (data, pixel_format) = match info.color_type {
        png::ColorType::Rgb => {
            let rgba_data = convert_rgb_data_to_rgba(&buf);
            (rgba_data, PixelFormat::Rgba8)
        }
        png::ColorType::Rgba => (buf, PixelFormat::Rgba8),
        png::ColorType::Grayscale => (buf, PixelFormat::Luma8),
        png::ColorType::GrayscaleAlpha => {
            let luma_data = convert_luma_alpha_data_to_luma(&buf);
            (luma_data, PixelFormat::Luma8)
        }
        png::ColorType::Indexed => bail!("Unsupported PNG color type: {:?}", info.color_type),
    };

    Ok(Image {
        meta: ImageMetadata {
            width,
            height,
            pixel_format,
        },
        data,
    })
}

/// Reads the metadata of a JPEG image from a byte buffer.
#[cfg(feature = "jpeg")]
fn read_jpeg_metadata_from_bytes(bytes: &[u8]) -> Result<ImageMetadata> {
    use zune_jpeg::zune_core::colorspace::ColorSpace;

    let mut decoder = zune_jpeg::JpegDecoder::new(bytes);
    decoder
        .decode_headers()
        .context("Failed to decode JPEG headers")?;

    let colorspace = decoder.get_output_colorspace().unwrap();

    let pixel_format = match colorspace {
        // Image data is converted to RGBA8 when read
        ColorSpace::RGB | ColorSpace::RGBA | ColorSpace::YCbCr => PixelFormat::Rgba8,
        // Alpha channel is stripped if present when image data is read
        ColorSpace::Luma | ColorSpace::LumaA => PixelFormat::Luma8,
        _ => bail!("Unsupported JPEG colorspace: {:?}", colorspace),
    };

    let (width, height) = decoder.dimensions().unwrap();

    Ok(ImageMetadata {
        width: width as u32,
        height: height as u32,
        pixel_format,
    })
}

/// Loads a JPEG image from a byte buffer.
#[cfg(feature = "jpeg")]
fn load_jpeg_from_bytes(bytes: &[u8]) -> Result<Image> {
    use zune_jpeg::zune_core::colorspace::ColorSpace;

    let mut decoder = zune_jpeg::JpegDecoder::new(bytes);
    let pixels = decoder.decode().context("Failed to decode JPEG")?;

    let colorspace = decoder.get_output_colorspace().unwrap();

    let (data, format) = match colorspace {
        ColorSpace::RGB => {
            let rgba_data = convert_rgb_data_to_rgba(&pixels);
            (rgba_data, PixelFormat::Rgba8)
        }
        ColorSpace::RGBA => (pixels, PixelFormat::Rgba8),
        ColorSpace::YCbCr => {
            let rgba_data = convert_ycbcr_data_to_rgba(&pixels);
            (rgba_data, PixelFormat::Rgba8)
        }
        ColorSpace::Luma => (pixels, PixelFormat::Luma8),
        ColorSpace::LumaA => {
            let luma_data = convert_luma_alpha_data_to_luma(&pixels);
            (luma_data, PixelFormat::Luma8)
        }
        _ => bail!("Unsupported JPEG colorspace: {:?}", colorspace),
    };

    let (width, height) = decoder.dimensions().unwrap();

    Ok(Image {
        meta: ImageMetadata {
            width: width as u32,
            height: height as u32,
            pixel_format: format,
        },
        data,
    })
}

/// Saves RGBA8 pixel data as a PNG file.
#[cfg(feature = "png")]
pub fn save_rgba8_as_png(
    data: &[u8],
    width: u32,
    height: u32,
    path: impl AsRef<Path>,
) -> Result<()> {
    let path = path.as_ref();
    impact_log::debug!("Saving color image as PNG to {}", path.display());

    let file = crate::create_file_and_required_directories(path)?;
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .context("Failed to write PNG header")?;

    writer
        .write_image_data(data)
        .context("Failed to write PNG image data")?;

    Ok(())
}

/// Saves Luma8 (grayscale) pixel data as a PNG file.
#[cfg(feature = "png")]
pub fn save_luma8_as_png(
    data: &[u8],
    width: u32,
    height: u32,
    path: impl AsRef<Path>,
) -> Result<()> {
    let path = path.as_ref();
    impact_log::debug!("Saving grayscale image as PNG to {}", path.display());

    let file = crate::create_file_and_required_directories(path)?;
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .context("Failed to write PNG header")?;

    writer
        .write_image_data(data)
        .context("Failed to write PNG image data")?;

    Ok(())
}

/// Saves a decoded image as a PNG file.
///
/// The image will be saved in its current format (RGBA8 or Luma8).
#[cfg(feature = "png")]
pub fn save_image_as_png(image: &Image, path: impl AsRef<Path>) -> Result<()> {
    match image.meta.pixel_format {
        PixelFormat::Rgba8 => {
            save_rgba8_as_png(&image.data, image.meta.width, image.meta.height, path)
        }
        PixelFormat::Luma8 => {
            save_luma8_as_png(&image.data, image.meta.width, image.meta.height, path)
        }
    }
}

fn convert_luma_data_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut rgba_data = Vec::with_capacity(data.len() * 4);
    for &luma in data {
        rgba_data.extend_from_slice(&[luma, luma, luma, 255]);
    }
    rgba_data
}

fn convert_rgb_data_to_rgba(data: &[u8]) -> Vec<u8> {
    let (rgb_data, rem) = data.as_chunks::<3>();
    assert!(rem.is_empty());

    let mut rgba_data = Vec::with_capacity((data.len() / 3) * 4);

    for &[r, g, b] in rgb_data {
        rgba_data.extend_from_slice(&[r, g, b, 255]);
    }
    rgba_data
}

fn convert_rgba_data_to_luma(data: &[u8]) -> Vec<u8> {
    let mut luma_data = Vec::with_capacity(data.len() / 4);

    let (rgba_data, rem) = data.as_chunks::<4>();
    assert!(rem.is_empty());

    for &[r, g, b, _] in rgba_data {
        luma_data.push(rgb_to_luma(r, g, b));
    }
    luma_data
}

/// Ignores alpha.
fn convert_luma_alpha_data_to_luma(data: &[u8]) -> Vec<u8> {
    let (luma_alpha_data, rem) = data.as_chunks::<2>();
    assert!(rem.is_empty());

    let mut luma_data = Vec::with_capacity(data.len() / 2);

    for &[luma, _] in luma_alpha_data {
        luma_data.push(luma);
    }
    luma_data
}

fn convert_ycbcr_data_to_rgba(data: &[u8]) -> Vec<u8> {
    let (ycbcr_data, rem) = data.as_chunks::<3>();
    assert!(rem.is_empty());

    let mut rgba_data = Vec::with_capacity((data.len() / 3) * 4);

    for &[y, cb, cr] in ycbcr_data {
        let (r, g, b) = ycbcr_to_rgb(y, cb, cr);
        rgba_data.extend_from_slice(&[r, g, b, 255]);
    }
    rgba_data
}

fn rgb_to_luma(r: u8, g: u8, b: u8) -> u8 {
    (0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)) as u8
}

fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
    let y = f32::from(y);
    let cb = f32::from(cb) - 128.0;
    let cr = f32::from(cr) - 128.0;

    // ITU-R BT.601 conversion coefficients
    let r = y + 1.402 * cr;
    let g = y - 0.34414 * cb - 0.71414 * cr;
    let b = y + 1.772 * cb;

    (
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoded_image_dimensions() {
        let image = Image {
            meta: ImageMetadata {
                width: 100,
                height: 200,
                pixel_format: PixelFormat::Rgba8,
            },
            data: vec![0; 100 * 200 * 4],
        };

        assert_eq!(image.dimensions(), (100, 200));
    }

    #[test]
    fn test_has_color() {
        let rgba_image = Image {
            meta: ImageMetadata {
                width: 1,
                height: 1,
                pixel_format: PixelFormat::Rgba8,
            },
            data: vec![255, 0, 0, 255],
        };

        let luma_image = Image {
            meta: ImageMetadata {
                width: 1,
                height: 1,
                pixel_format: PixelFormat::Luma8,
            },
            data: vec![128],
        };

        assert!(rgba_image.has_color());
        assert!(!luma_image.has_color());
    }

    #[test]
    fn test_luma_to_rgba_conversion() {
        let luma_image = Image {
            meta: ImageMetadata {
                width: 2,
                height: 1,
                pixel_format: PixelFormat::Luma8,
            },
            data: vec![100, 200],
        };

        let rgba_data = luma_image.into_rgba8();
        assert_eq!(rgba_data, vec![100, 100, 100, 255, 200, 200, 200, 255]);
    }

    #[test]
    fn test_rgba_to_luma_conversion() {
        let rgba_image = Image {
            meta: ImageMetadata {
                width: 1,
                height: 1,
                pixel_format: PixelFormat::Rgba8,
            },
            data: vec![255, 0, 0, 255], // Red pixel
        };

        let luma_data = rgba_image.into_luma8();
        // 0.299 * 255 = 76.245, rounded to 76
        assert_eq!(luma_data, vec![76]);
    }

    #[test]
    fn test_rgb_to_rgba_conversion() {
        let rgb_data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255];
        let rgba_data = super::convert_rgb_data_to_rgba(&rgb_data);
        assert_eq!(
            rgba_data,
            vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255]
        );
    }

    #[test]
    fn test_luma_alpha_to_luma_conversion() {
        let luma_alpha_data = vec![100, 200, 150, 50]; // Two pixels with alpha
        let luma_data = super::convert_luma_alpha_data_to_luma(&luma_alpha_data);
        assert_eq!(luma_data, vec![100, 150]); // Alpha values ignored
    }

    #[test]
    fn test_ycbcr_to_rgb_conversion() {
        // Test conversion of white color (Y=255, Cb=128, Cr=128)
        let (r, g, b) = super::ycbcr_to_rgb(255, 128, 128);
        assert_eq!((r, g, b), (255, 255, 255));

        // Test conversion of black color (Y=0, Cb=128, Cr=128)
        let (r, g, b) = super::ycbcr_to_rgb(0, 128, 128);
        assert_eq!((r, g, b), (0, 0, 0));

        // Test conversion of pure red in YCbCr space
        // Red (255,0,0) -> YCbCr should be approximately (76, 85, 255)
        let (r, g, b) = super::ycbcr_to_rgb(76, 85, 255);
        // Red should be dominant, green and blue should be minimal
        assert!(r > 200);
        assert!(g < 50);
        assert!(b < 50);

        // Test conversion of pure green in YCbCr space
        // Green (0,255,0) -> YCbCr should be approximately (150, 44, 21)
        let (r, g, b) = super::ycbcr_to_rgb(150, 44, 21);
        // Green should be dominant
        assert!(g > 200);
        assert!(r < 50);
        assert!(b < 50);

        // Test conversion of pure blue in YCbCr space
        // Blue (0,0,255) -> YCbCr should be approximately (29, 255, 107)
        let (r, g, b) = super::ycbcr_to_rgb(29, 255, 107);
        // Blue should be dominant
        assert!(b > 200);
        assert!(r < 50);
        assert!(g < 50);
    }

    #[test]
    fn test_ycbcr_data_to_rgba_conversion() {
        // Test YCbCr data for white and black pixels
        let ycbcr_data = vec![
            255, 128, 128, // White
            0, 128, 128, // Black
        ];
        let rgba_data = super::convert_ycbcr_data_to_rgba(&ycbcr_data);

        // Check white pixel (first 4 bytes)
        assert_eq!(&rgba_data[0..4], &[255, 255, 255, 255]);

        // Check black pixel (next 4 bytes)
        assert_eq!(&rgba_data[4..8], &[0, 0, 0, 255]);
    }

    #[test]
    fn test_unsupported_format_detection() {
        let fake_data = b"This is not an image";
        let result = load_image_from_bytes(fake_data);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported image format")
        );
    }
}
