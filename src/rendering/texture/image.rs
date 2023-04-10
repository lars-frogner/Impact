//! Textures representing 2D images.

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, bail, Result};
use image::io::Reader as ImageReader;
use image::{self, DynamicImage, GenericImageView};
use std::{num::NonZeroU32, path::Path};

/// A texture representing a 2D image.
#[derive(Debug)]
pub struct ImageTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

/// Configuration for [`ImageTexture`]s.
#[derive(Clone, Debug)]
pub struct ImageTextureConfig {
    /// The color space that the pixel values should be assumed to be stored in.
    pub color_space: ColorSpace,
    /// How addressing outside the [0, 1] range for the U texture coordinate
    /// should be handled.
    pub address_mode_u: wgpu::AddressMode,
    /// How addressing outside the [0, 1] range for the V texture coordinate
    /// should be handled.
    pub address_mode_v: wgpu::AddressMode,
}

impl ImageTextureConfig {
    pub const REPEATING_COLOR_TEXTRUE: Self = Self {
        color_space: ColorSpace::Srgb,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
    };

    pub const REPEATING_NON_COLOR_TEXTRUE: Self = Self {
        color_space: ColorSpace::Linear,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
    };
}

/// A color space for pixel values.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    Linear,
    Srgb,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ColorType {
    Rgba8(ColorSpace),
    Grayscale8,
}

impl ImageTexture {
    /// Creates a texture for the image file at the given path, using the given
    /// configuration parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image file can not be read or decoded.
    /// - The image bytes can not be interpreted.
    /// - The image width or height is zero.
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_path(
        core_system: &CoreRenderingSystem,
        image_path: impl AsRef<Path>,
        config: ImageTextureConfig,
    ) -> Result<Self> {
        let image_path = image_path.as_ref();
        let image = ImageReader::open(image_path)?.decode()?;
        Self::from_image(core_system, image, config, &image_path.to_string_lossy())
    }

    /// Creates a texture for the image file represented by the given raw byte
    /// buffer, using the given configuration parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image bytes can not be interpreted.
    /// - The image width or height is zero.
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_bytes(
        core_system: &CoreRenderingSystem,
        byte_buffer: &[u8],
        config: ImageTextureConfig,
        label: &str,
    ) -> Result<Self> {
        let image = image::load_from_memory(byte_buffer)?;
        Self::from_image(core_system, image, config, label)
    }

    /// Creates a texture for the given loaded image, using the given
    /// configuration parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image width or height is zero.
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_image(
        core_system: &CoreRenderingSystem,
        image: DynamicImage,
        config: ImageTextureConfig,
        label: &str,
    ) -> Result<Self> {
        let (width, height) = image.dimensions();
        let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
        let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;

        Ok(if image.color().has_color() {
            Self::new(
                core_system,
                &image.into_rgba8(),
                width,
                height,
                ColorType::Rgba8(config.color_space),
                config,
                label,
            )
        } else {
            if config.color_space != ColorSpace::Linear {
                bail!(
                    "Unsupported color space {:?} for grayscale image {}",
                    config.color_space,
                    label
                );
            }
            Self::new(
                core_system,
                &image.into_luma8(),
                width,
                height,
                ColorType::Grayscale8,
                config,
                label,
            )
        })
    }

    /// Creates a texture for the image represented by the given byte buffer,
    /// dimensions and color type, using the given configuration parameters.
    fn new(
        core_system: &CoreRenderingSystem,
        byte_buffer: &[u8],
        width: NonZeroU32,
        height: NonZeroU32,
        color_type: ColorType,
        config: ImageTextureConfig,
        label: &str,
    ) -> Self {
        let device = core_system.device();

        let texture_size = wgpu::Extent3d {
            width: u32::from(width),
            height: u32::from(height),
            depth_or_array_layers: 1,
        };

        let texture = Self::create_empty_image_texture(device, color_type, texture_size, label);
        Self::write_data_to_image_texture(
            core_system.queue(),
            &texture,
            byte_buffer,
            color_type,
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = Self::create_sampler(device, config.address_mode_u, config.address_mode_v);

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    /// Returns a view into the image texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns a sampler for the image texture.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Creates the bind group layout entry for this texture type, assigned to
    /// the given binding.
    pub const fn create_texture_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }
    }

    /// Creates the bind group layout entry for this texture's sampler type,
    /// assigned to the given binding.
    pub const fn create_sampler_bind_group_layout_entry(
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            // The sampler binding type must be consistent with the `filterable`
            // field in the texture sample type.
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }
    }

    /// Creates the bind group entry for this texture, assigned to the given
    /// binding.
    pub fn create_texture_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(self.view()),
        }
    }

    /// Creates the bind group entry for this texture's sampler, assigned to the
    /// given binding.
    pub fn create_sampler_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(self.sampler()),
        }
    }

    /// Creates a new [`wgpu::Texture`] configured to hold 2D image data.
    fn create_empty_image_texture(
        device: &wgpu::Device,
        color_type: ColorType,
        texture_size: wgpu::Extent3d,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: color_type.texture_format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[],
        })
    }

    fn write_data_to_image_texture(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        byte_buffer: &[u8],
        color_type: ColorType,
        texture_size: wgpu::Extent3d,
    ) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            byte_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(color_type.n_bytes() * texture_size.width),
                rows_per_image: NonZeroU32::new(texture_size.height),
            },
            texture_size,
        );
    }

    fn create_sampler(
        device: &wgpu::Device,
        address_mode_u: wgpu::AddressMode,
        address_mode_v: wgpu::AddressMode,
    ) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u,
            address_mode_v,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
    }
}

impl ColorType {
    fn n_bytes(&self) -> u32 {
        match self {
            Self::Rgba8(_) => 4,
            Self::Grayscale8 => 1,
        }
    }

    fn texture_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8(ColorSpace::Linear) => wgpu::TextureFormat::Rgba8Unorm,
            Self::Rgba8(ColorSpace::Srgb) => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Grayscale8 => wgpu::TextureFormat::R8Unorm,
        }
    }
}
