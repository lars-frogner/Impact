//! Processing of imported textures.

use allocator_api2::{alloc::Allocator, vec::Vec as AVec};
use impact_io::image::{Image, ImageMetadata, PixelFormat};

/// Processing operations for a texture image.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct ImageProcessing {
    /// Format conversion operations.
    pub format_conversions: Vec<FormatConversion>,
}

/// Conversion between texture representations.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum FormatConversion {
    NormalMap { from: NormalMapFormat },
}

/// The convention used for storing normal vectors in a texture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NormalMapFormat {
    /// DirectX convention - larger G value is larger y-component.
    #[default]
    DirectX,
    /// OpenGL convention - larger G value is smaller y-component.
    OpenGL,
}

impl ImageProcessing {
    pub fn none() -> Self {
        Self {
            format_conversions: Vec::new(),
        }
    }

    /// Executes the processing operations on the given image, returning the
    /// processed image or [`None`] if there were no operations to perform.
    pub fn execute<A, IA>(&self, arena: A, image: &Image<IA>) -> Option<Image<A>>
    where
        A: Copy + Allocator,
        IA: Allocator,
    {
        if self.is_none_for(&image.meta) {
            return None;
        }

        let mut data = AVec::with_capacity_in(image.data.len(), arena);
        data.extend_from_slice(&image.data);

        for conversion in &self.format_conversions {
            conversion.apply(&image.meta, &mut data);
        }

        Some(Image {
            meta: image.meta.clone(),
            data,
        })
    }

    /// Executes the processing operations on the given image bytes.
    pub fn execute_in_place(&self, meta: &ImageMetadata, data: &mut [u8]) {
        if self.is_none_for(meta) {
            return;
        }

        for conversion in &self.format_conversions {
            conversion.apply(meta, data);
        }
    }

    fn is_none_for(&self, meta: &ImageMetadata) -> bool {
        for conversion in &self.format_conversions {
            match conversion {
                FormatConversion::NormalMap { from } => {
                    // No normal map conversion required if format is already
                    // DirectX
                    if *from == NormalMapFormat::DirectX {
                        continue;
                    }
                    // Normal map conversion doesn't make sense for grayscale
                    // images
                    if meta.pixel_format == PixelFormat::Luma8 {
                        continue;
                    }
                }
            }
            return false;
        }
        true
    }
}

impl FormatConversion {
    fn apply(&self, meta: &ImageMetadata, data: &mut [u8]) {
        match self {
            Self::NormalMap { from } => match from {
                NormalMapFormat::OpenGL => {
                    convert_normal_map_format_from_opengl_to_directx(meta, data);
                }
                NormalMapFormat::DirectX => {}
            },
        }
    }
}

fn convert_normal_map_format_from_opengl_to_directx(meta: &ImageMetadata, data: &mut [u8]) {
    match meta.pixel_format {
        PixelFormat::Rgba8 => {
            // Invert green channel (index 1 in RGBA)
            for pixel in data.chunks_exact_mut(4) {
                pixel[1] = 255 - pixel[1];
            }
        }
        PixelFormat::Luma8 => {}
    }
}
