//! Textures management.

pub mod io;
pub mod resource;

use anyhow::{Result, anyhow, bail};
use impact_gpu::{
    device::GraphicsDevice,
    texture::{
        ColorSpace, DepthOrArrayLayers, SamplerID, TexelDescription, Texture, TextureConfig,
        mipmap::MipmapperGenerator,
    },
};
use impact_io::image::{self, Image, PixelFormat};
use impact_math::{hash32, stringhash32_newtype};
use roc_integration::roc;
use std::{hash::Hash, num::NonZeroU32};

stringhash32_newtype!(
    /// Identifier for specific textures.
    /// Wraps a [`StringHash32`](impact_math::StringHash32).
    #[roc(parents = "Rendering")]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    [pub] TextureID
);

#[roc(dependencies = [impact_math::Hash32])]
impl TextureID {
    #[roc(body = "Hashing.hash_str_32(name)")]
    /// Creates a texture ID hashed from the given name.
    pub fn from_name(name: &str) -> Self {
        Self(hash32!(name))
    }
}

/// Creates a texture for the image file represented by the given raw byte
/// buffer, using the given configuration parameters. Mipmaps will be
/// generated automatically.
///
/// # Errors
/// Returns an error if:
/// - The image bytes can not be interpreted.
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_texture_from_bytes(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    byte_buffer: &[u8],
    texture_config: TextureConfig,
    sampler_id: Option<SamplerID>,
    label: &str,
) -> Result<Texture> {
    let image = image::load_image_from_bytes(byte_buffer)?;
    create_texture_from_image(
        graphics_device,
        mipmapper_generator,
        image,
        texture_config,
        sampler_id,
        label,
    )
}

/// Creates a texture for the given loaded image, using the given
/// configuration parameters. Mipmaps will be generated automatically.
///
/// # Errors
/// Returns an error if:
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_texture_from_image(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    image: Image,
    texture_config: TextureConfig,
    sampler_id: Option<SamplerID>,
    label: &str,
) -> Result<Texture> {
    let (width, height) = image.dimensions();
    let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;
    let depth = NonZeroU32::new(1).unwrap();

    match image.pixel_format {
        PixelFormat::Rgba8 => Texture::create(
            graphics_device,
            Some(mipmapper_generator),
            &image.data,
            width,
            height,
            DepthOrArrayLayers::Depth(depth),
            TexelDescription::Rgba8(texture_config.color_space),
            false,
            texture_config,
            sampler_id,
            label,
        ),
        PixelFormat::Luma8 => {
            if texture_config.color_space != ColorSpace::Linear {
                bail!(
                    "Unsupported color space {:?} for grayscale image {}",
                    texture_config.color_space,
                    label
                );
            }
            Texture::create(
                graphics_device,
                Some(mipmapper_generator),
                &image.data,
                width,
                height,
                DepthOrArrayLayers::Depth(depth),
                TexelDescription::Grayscale8,
                false,
                texture_config,
                sampler_id,
                label,
            )
        }
    }
}

/// Creates a cubemap texture for the given loaded images representing
/// cubemap faces, using the given configuration parameters.
///
/// # Errors
/// Returns an error if:
/// - The image dimensions or pixel formats do not match.
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_cubemap_texture_from_images(
    graphics_device: &GraphicsDevice,
    right_image: Image,
    left_image: Image,
    top_image: Image,
    bottom_image: Image,
    front_image: Image,
    back_image: Image,
    texture_config: TextureConfig,
    sampler_id: Option<SamplerID>,
    label: &str,
) -> Result<Texture> {
    let dimensions = right_image.dimensions();
    if left_image.dimensions() != dimensions
        || top_image.dimensions() != dimensions
        || bottom_image.dimensions() != dimensions
        || front_image.dimensions() != dimensions
        || back_image.dimensions() != dimensions
    {
        bail!("Inconsistent dimensions for cubemap texture images")
    }

    let pixel_format = right_image.pixel_format;
    if left_image.pixel_format != pixel_format
        || top_image.pixel_format != pixel_format
        || bottom_image.pixel_format != pixel_format
        || front_image.pixel_format != pixel_format
        || back_image.pixel_format != pixel_format
    {
        bail!("Inconsistent pixel formats for cubemap texture images")
    }

    let (width, height) = right_image.dimensions();
    let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;
    let array_layers = NonZeroU32::new(6).unwrap();

    let (texel_description, byte_buffer) = match pixel_format {
        PixelFormat::Rgba8 => {
            let texel_description = TexelDescription::Rgba8(texture_config.color_space);

            let mut byte_buffer = Vec::with_capacity(
                (6 * dimensions.0 * dimensions.1 * texel_description.n_bytes()) as usize,
            );

            byte_buffer.extend_from_slice(&right_image.data);
            byte_buffer.extend_from_slice(&left_image.data);
            byte_buffer.extend_from_slice(&top_image.data);
            byte_buffer.extend_from_slice(&bottom_image.data);
            byte_buffer.extend_from_slice(&front_image.data);
            byte_buffer.extend_from_slice(&back_image.data);

            (texel_description, byte_buffer)
        }
        PixelFormat::Luma8 => {
            if texture_config.color_space != ColorSpace::Linear {
                bail!(
                    "Unsupported color space {:?} for grayscale image {}",
                    texture_config.color_space,
                    label
                );
            }

            let texel_description = TexelDescription::Grayscale8;

            let mut byte_buffer = Vec::with_capacity(
                (6 * dimensions.0 * dimensions.1 * texel_description.n_bytes()) as usize,
            );

            byte_buffer.extend_from_slice(&right_image.data);
            byte_buffer.extend_from_slice(&left_image.data);
            byte_buffer.extend_from_slice(&top_image.data);
            byte_buffer.extend_from_slice(&bottom_image.data);
            byte_buffer.extend_from_slice(&front_image.data);
            byte_buffer.extend_from_slice(&back_image.data);

            (texel_description, byte_buffer)
        }
    };

    Texture::create(
        graphics_device,
        None,
        &byte_buffer,
        width,
        height,
        DepthOrArrayLayers::ArrayLayers(array_layers),
        texel_description,
        true,
        texture_config,
        sampler_id,
        label,
    )
}

/// Creates a texture array for the given loaded images, using the given
/// configuration parameters.
///
/// # Errors
/// Returns an error if:
/// - The number of images is zero.
/// - Any of the images are wrapped in an [`Err`].
/// - The image width or height is zero.
/// - The image is grayscale and the color space in the configuration is not
///   linear.
/// - The image dimensions or pixel formats do not match.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
pub fn create_texture_array_from_images<I>(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    images: impl IntoIterator<IntoIter = I>,
    texture_config: TextureConfig,
    sampler_id: Option<SamplerID>,
    label: &str,
) -> Result<Texture>
where
    I: ExactSizeIterator<Item = Result<Image>>,
{
    let mut images = images.into_iter();
    let n_images = images.len();

    let first_image = images
        .next()
        .ok_or_else(|| anyhow!("No images for texture array"))??;

    let dimensions = first_image.dimensions();
    let width = NonZeroU32::new(dimensions.0).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(dimensions.1).ok_or_else(|| anyhow!("Image height is zero"))?;
    let array_layers = NonZeroU32::new(u32::try_from(n_images).unwrap()).unwrap();

    let pixel_format = first_image.pixel_format;
    let texel_description = match pixel_format {
        PixelFormat::Rgba8 => TexelDescription::Rgba8(texture_config.color_space),
        PixelFormat::Luma8 => {
            if texture_config.color_space != ColorSpace::Linear {
                bail!(
                    "Unsupported color space {:?} for grayscale image {}",
                    texture_config.color_space,
                    label
                );
            }
            TexelDescription::Grayscale8
        }
    };

    let mut byte_buffer = Vec::with_capacity(
        n_images * (width.get() * height.get() * texel_description.n_bytes()) as usize,
    );

    byte_buffer.extend_from_slice(&first_image.data);

    for image in images {
        let image = image?;

        if image.dimensions() != dimensions {
            bail!("Inconsistent dimensions for texture array images")
        }

        if image.pixel_format != pixel_format {
            bail!("Inconsistent pixel formats for texture array images")
        }

        byte_buffer.extend_from_slice(&image.data);
    }

    Texture::create(
        graphics_device,
        Some(mipmapper_generator),
        &byte_buffer,
        width,
        height,
        DepthOrArrayLayers::ArrayLayers(array_layers),
        texel_description,
        false,
        texture_config,
        sampler_id,
        label,
    )
}
