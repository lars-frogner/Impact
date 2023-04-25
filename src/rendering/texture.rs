//! Textures.

mod attachment;
mod shadow_map;

pub use attachment::{
    RenderAttachmentQuantity, RenderAttachmentQuantitySet, RenderAttachmentTextureManager,
    RENDER_ATTACHMENT_BINDINGS, RENDER_ATTACHMENT_FLAGS, RENDER_ATTACHMENT_FORMATS,
};
pub use shadow_map::{
    CascadeIdx, CascadedShadowMapTexture, ShadowCubemapTexture, SHADOW_MAP_FORMAT,
};

use crate::{rendering::CoreRenderingSystem, scene};
use anyhow::{anyhow, bail, Result};
use bytemuck::Pod;
use image::{
    self, buffer::ConvertBuffer, io::Reader as ImageReader, DynamicImage, GenericImageView,
    ImageBuffer, Luma, Rgba,
};
use rmp_serde::{from_read, Serializer};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    borrow::Cow, collections::HashMap, fs::File, io::BufReader, num::NonZeroU32, path::Path,
};
use wgpu::util::DeviceExt;

/// Represents a data type that can be copied directly to a [`Texture`].
pub trait TexelType: Pod {
    const DESCRIPTION: TexelDescription;
}

/// A description of the data type used for a texel.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TexelDescription {
    Rgba8(ColorSpace),
    Grayscale8,
    Float32,
}

/// A color space for pixel/texel values.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorSpace {
    Linear,
    Srgb,
}

/// A texture holding multidimensional data.
#[derive(Debug)]
pub struct Texture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    view_dimension: wgpu::TextureViewDimension,
}

/// Configuration parameters for [`Texture`]s.
#[derive(Clone, Debug, Default)]
pub struct TextureConfig {
    /// The color space that the texel data values should be assumed to be
    /// stored in.
    pub color_space: ColorSpace,
    /// Configuration for texture addressing.
    pub addressing: TextureAddressingConfig,
    /// Configuration for texture filtering.
    pub filtering: TextureFilteringConfig,
}

/// Configuration parameters for addressing [`Texture`]s.
#[derive(Clone, Debug)]
pub struct TextureAddressingConfig {
    /// How addressing outside the [0, 1] range for the U texture coordinate
    /// should be handled.
    pub address_mode_u: wgpu::AddressMode,
    /// How addressing outside the [0, 1] range for the V texture coordinate
    /// should be handled.
    pub address_mode_v: wgpu::AddressMode,
    /// How addressing outside the [0, 1] range for the W texture coordinate
    /// should be handled.
    pub address_mode_w: wgpu::AddressMode,
}

/// Configuration parameters for filtering of [`Texture`]s.
#[derive(Clone, Debug)]
pub struct TextureFilteringConfig {
    /// How to filter the texture when it needs to be magnified.
    pub mag_filter: wgpu::FilterMode,
    /// How to filter the texture when it needs to be minified.
    pub min_filter: wgpu::FilterMode,
    /// The maximum number of mip levels that should be generated for the
    /// texture. If [`None`], a full mipmap chain will be generated.
    pub max_mip_level_count: Option<u32>,
    /// How to filter between mipmap levels.
    pub mipmap_filter: wgpu::FilterMode,
    /// Minimum level of detail (i.e. mip level) to use.
    pub lod_min_clamp: f32,
    /// Maximum level of detail (i.e. mip level) to use.
    pub lod_max_clamp: f32,
    /// Maximum number of samples to use for anisotropic filtering.
    pub anisotropy_clamp: u16,
}

/// Dimensions and data for a lookup table to be loaded into a texture.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextureLookupTable<T: TexelType> {
    width: NonZeroU32,
    height: NonZeroU32,
    depth_or_array_layers: DepthOrArrayLayers,
    data: Vec<T>,
}

/// A number that either represents the number of depths in a 3D texture or the
/// number of layers in a 1D or 2D texture array.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepthOrArrayLayers {
    Depth(NonZeroU32),
    ArrayLayers(NonZeroU32),
}

/// Helper for generating mipmaps for a texture.
#[derive(Debug)]
pub struct MipmapGenerator {
    _shader: wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    pipelines_and_bind_group_layouts:
        HashMap<wgpu::TextureFormat, (wgpu::RenderPipeline, wgpu::BindGroupLayout)>,
}

impl Default for ColorSpace {
    fn default() -> Self {
        Self::Linear
    }
}

impl TexelDescription {
    const AVAILABLE_FORMATS: [wgpu::TextureFormat; 4] = [
        wgpu::TextureFormat::R8Unorm,
        wgpu::TextureFormat::R32Float,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ];

    fn n_bytes(&self) -> u32 {
        match self {
            Self::Rgba8(_) | Self::Float32 => 4,
            Self::Grayscale8 => 1,
        }
    }

    fn texture_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8(ColorSpace::Linear) => wgpu::TextureFormat::Rgba8Unorm,
            Self::Rgba8(ColorSpace::Srgb) => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Grayscale8 => wgpu::TextureFormat::R8Unorm,
            Self::Float32 => wgpu::TextureFormat::R32Float,
        }
    }
}

impl TexelType for f32 {
    const DESCRIPTION: TexelDescription = TexelDescription::Float32;
}

impl TexelType for u8 {
    const DESCRIPTION: TexelDescription = TexelDescription::Grayscale8;
}

impl Texture {
    /// Creates a texture for the image file at the given path, using the given
    /// configuration parameters. Mipmaps will be generated automatically.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image file can not be read or decoded.
    /// - The image bytes can not be interpreted.
    /// - The image width or height is zero.
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_path(
        core_system: &CoreRenderingSystem,
        mipmap_generator: &MipmapGenerator,
        image_path: impl AsRef<Path>,
        config: TextureConfig,
    ) -> Result<Self> {
        let image_path = image_path.as_ref();
        let image = ImageReader::open(image_path)?.decode()?;
        Self::from_image(
            core_system,
            mipmap_generator,
            image,
            config,
            &image_path.to_string_lossy(),
        )
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
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_bytes(
        core_system: &CoreRenderingSystem,
        mipmap_generator: &MipmapGenerator,
        byte_buffer: &[u8],
        config: TextureConfig,
        label: &str,
    ) -> Result<Self> {
        let image = image::load_from_memory(byte_buffer)?;
        Self::from_image(core_system, mipmap_generator, image, config, label)
    }

    /// Creates a texture for the given loaded image, using the given
    /// configuration parameters. Mipmaps will be generated automatically.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image width or height is zero.
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_image(
        core_system: &CoreRenderingSystem,
        mipmap_generator: &MipmapGenerator,
        image: DynamicImage,
        config: TextureConfig,
        label: &str,
    ) -> Result<Self> {
        let (width, height) = image.dimensions();
        let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
        let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;
        let depth = NonZeroU32::new(1).unwrap();

        if image.color().has_color() {
            Self::new(
                core_system,
                Some(mipmap_generator),
                &image.into_rgba8(),
                width,
                height,
                DepthOrArrayLayers::Depth(depth),
                TexelDescription::Rgba8(config.color_space),
                false,
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
                Some(mipmap_generator),
                &image.into_luma8(),
                width,
                height,
                DepthOrArrayLayers::Depth(depth),
                TexelDescription::Grayscale8,
                false,
                config,
                label,
            )
        }
    }
    /// Creates a cubemap texture for the image files representing cubemap faces
    /// at the given paths, using the given configuration parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image dimensions or pixel formats do not match.
    /// - The image file can not be read or decoded.
    /// - The image bytes can not be interpreted.
    /// - The image width or height is zero.
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_cubemap_image_paths<P: AsRef<Path>>(
        core_system: &CoreRenderingSystem,
        right_image_path: P,
        left_image_path: P,
        top_image_path: P,
        bottom_image_path: P,
        front_image_path: P,
        back_image_path: P,
        config: TextureConfig,
    ) -> Result<Self> {
        let right_image_path = right_image_path.as_ref();
        let left_image_path = left_image_path.as_ref();
        let top_image_path = top_image_path.as_ref();
        let bottom_image_path = bottom_image_path.as_ref();
        let front_image_path = front_image_path.as_ref();
        let back_image_path = back_image_path.as_ref();

        let right_image = ImageReader::open(right_image_path)?.decode()?;
        let left_image = ImageReader::open(left_image_path)?.decode()?;
        let top_image = ImageReader::open(top_image_path)?.decode()?;
        let bottom_image = ImageReader::open(bottom_image_path)?.decode()?;
        let front_image = ImageReader::open(front_image_path)?.decode()?;
        let back_image = ImageReader::open(back_image_path)?.decode()?;

        let label = format!(
            "Cubemap {{{}, {}, {}, {}, {}, {}}}",
            right_image_path.to_string_lossy(),
            left_image_path.to_string_lossy(),
            top_image_path.to_string_lossy(),
            bottom_image_path.to_string_lossy(),
            front_image_path.to_string_lossy(),
            back_image_path.to_string_lossy()
        );

        Self::from_cubemap_images(
            core_system,
            right_image,
            left_image,
            top_image,
            bottom_image,
            front_image,
            back_image,
            config,
            &label,
        )
    }

    /// Creates a cubemap texture for the given loaded images representing
    /// cubemap faces, using the given configuration parameters.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image dimensions or pixel formats do not match.
    /// - The image width or height is zero.
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_cubemap_images(
        core_system: &CoreRenderingSystem,
        right_image: DynamicImage,
        left_image: DynamicImage,
        top_image: DynamicImage,
        bottom_image: DynamicImage,
        front_image: DynamicImage,
        back_image: DynamicImage,
        config: TextureConfig,
        label: &str,
    ) -> Result<Self> {
        let dimensions = right_image.dimensions();
        if left_image.dimensions() != dimensions
            || top_image.dimensions() != dimensions
            || bottom_image.dimensions() != dimensions
            || front_image.dimensions() != dimensions
            || back_image.dimensions() != dimensions
        {
            bail!("Inconsistent dimensions for cubemap texture images")
        }

        let color = right_image.color();
        if left_image.color() != color
            || top_image.color() != color
            || bottom_image.color() != color
            || front_image.color() != color
            || back_image.color() != color
        {
            bail!("Inconsistent pixel formats for cubemap texture images")
        }

        let (width, height) = right_image.dimensions();
        let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
        let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;
        let array_layers = NonZeroU32::new(6).unwrap();

        let (texel_description, byte_buffer) = if color.has_color() {
            let texel_description = TexelDescription::Rgba8(config.color_space);

            let mut byte_buffer = Vec::with_capacity(
                (6 * dimensions.0 * dimensions.1 * texel_description.n_bytes()) as usize,
            );

            byte_buffer.extend_from_slice(&right_image.into_rgba8());
            byte_buffer.extend_from_slice(&left_image.into_rgba8());
            byte_buffer.extend_from_slice(&top_image.into_rgba8());
            byte_buffer.extend_from_slice(&bottom_image.into_rgba8());
            byte_buffer.extend_from_slice(&front_image.into_rgba8());
            byte_buffer.extend_from_slice(&back_image.into_rgba8());

            (texel_description, byte_buffer)
        } else {
            if config.color_space != ColorSpace::Linear {
                bail!(
                    "Unsupported color space {:?} for grayscale image {}",
                    config.color_space,
                    label
                );
            }

            let texel_description = TexelDescription::Grayscale8;

            let mut byte_buffer = Vec::with_capacity(
                (6 * dimensions.0 * dimensions.1 * texel_description.n_bytes()) as usize,
            );

            byte_buffer.extend_from_slice(&right_image.into_luma8());
            byte_buffer.extend_from_slice(&left_image.into_luma8());
            byte_buffer.extend_from_slice(&top_image.into_luma8());
            byte_buffer.extend_from_slice(&bottom_image.into_luma8());
            byte_buffer.extend_from_slice(&front_image.into_luma8());
            byte_buffer.extend_from_slice(&back_image.into_luma8());

            (texel_description, byte_buffer)
        };

        Self::new(
            core_system,
            None,
            &byte_buffer,
            width,
            height,
            DepthOrArrayLayers::ArrayLayers(array_layers),
            texel_description,
            true,
            config,
            label,
        )
    }

    /// Creates a texture holding the given lookup table. The texture will be
    /// sampled with (bi/tri)linear interpolation, and lookups outside [0, 1]
    /// are clamped to the edge values.
    ///
    /// # Errors
    /// Returns an error if the row size (width times data value size) is not a
    /// multiple of 256 bytes (`wgpu` requires that rows are a multiple of 256
    /// bytes for for copying data between buffers and textures).
    pub fn from_lookup_table<T: TexelType>(
        core_system: &CoreRenderingSystem,
        table: &TextureLookupTable<T>,
        label: &str,
    ) -> Result<Self> {
        let byte_buffer = bytemuck::cast_slice(&table.data);

        let config = TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::CLAMPED,
            filtering: TextureFilteringConfig::LOOKUP,
        };

        Self::new(
            core_system,
            None,
            byte_buffer,
            table.width,
            table.height,
            table.depth_or_array_layers,
            T::DESCRIPTION,
            false,
            config,
            label,
        )
    }

    /// Creates a texture for the data contained in the given byte buffer, with
    /// the given dimensions and texel description, using the given
    /// configuration parameters. Mipmaps will be generated automatically if a
    /// generator is provided.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The texture shape and texel size are inconsistent with the size of the
    ///   byte buffer.
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    fn new(
        core_system: &CoreRenderingSystem,
        mipmap_generator: Option<&MipmapGenerator>,
        byte_buffer: &[u8],
        width: NonZeroU32,
        height: NonZeroU32,
        depth_or_array_layers: DepthOrArrayLayers,
        texel_description: TexelDescription,
        is_cubemap: bool,
        config: TextureConfig,
        label: &str,
    ) -> Result<Self> {
        let texture_size = wgpu::Extent3d {
            width: u32::from(width),
            height: u32::from(height),
            depth_or_array_layers: u32::from(depth_or_array_layers.unwrap()),
        };

        if (texel_description.n_bytes()
            * texture_size.width
            * texture_size.height
            * texture_size.depth_or_array_layers) as usize
            != byte_buffer.len()
        {
            bail!(
                "Texture {} shape ({}, {}, {:?}) and texel size ({} bytes) not consistent with number bytes of data ({})",
                label,
                width,
                height,
                depth_or_array_layers,
                texel_description.n_bytes(),
                byte_buffer.len()
            )
        } else if (texel_description.n_bytes() * texture_size.width) % 256 != 0 {
            bail!(
                "Texture {} row size ({} bytes) is not a multiple of 256 bytes",
                label,
                texel_description.n_bytes() * texture_size.width
            )
        }

        let (dimension, view_dimension) =
            if let DepthOrArrayLayers::ArrayLayers(array_layers) = depth_or_array_layers {
                let view_dimension = if is_cubemap {
                    if array_layers.get() != 6 {
                        bail!("Tried to create cubemap with {} array layers", array_layers);
                    }
                    wgpu::TextureViewDimension::Cube
                } else {
                    wgpu::TextureViewDimension::D2Array
                };
                (wgpu::TextureDimension::D2, view_dimension)
            } else if texture_size.depth_or_array_layers == 1 {
                if texture_size.height == 1 {
                    (wgpu::TextureDimension::D1, wgpu::TextureViewDimension::D1)
                } else {
                    (wgpu::TextureDimension::D2, wgpu::TextureViewDimension::D2)
                }
            } else {
                (wgpu::TextureDimension::D3, wgpu::TextureViewDimension::D3)
            };

        let device = core_system.device();
        let queue = core_system.queue();

        let format = texel_description.texture_format();

        let full_mip_chain_level_count = ((u32::max(texture_size.width, texture_size.height) as f32)
            .log2()
            .ceil() as u32)
            + 1;

        let mip_level_count = if mipmap_generator.is_some() {
            u32::max(
                1,
                u32::min(
                    full_mip_chain_level_count,
                    config
                        .filtering
                        .max_mip_level_count
                        .unwrap_or(full_mip_chain_level_count),
                ),
            )
        } else {
            1
        };

        let texture = Self::create_empty_texture(
            device,
            format,
            texture_size,
            dimension,
            mip_level_count,
            label,
        );

        Self::write_data_to_texture(
            queue,
            &texture,
            byte_buffer,
            texel_description,
            texture_size,
        );

        if mip_level_count > 1 {
            if let Some(mipmap_generator) = mipmap_generator {
                mipmap_generator.generate_mipmaps(
                    device,
                    queue,
                    &texture,
                    format,
                    mip_level_count,
                    label,
                );
            }
        }

        let view = Self::create_view(&texture, view_dimension);

        let sampler = Self::create_sampler(
            device,
            config.addressing.address_mode_u,
            config.addressing.address_mode_v,
            config.addressing.address_mode_w,
            config.filtering.mag_filter,
            config.filtering.min_filter,
            config.filtering.mipmap_filter,
            config.filtering.lod_min_clamp,
            config.filtering.lod_max_clamp,
            config.filtering.anisotropy_clamp,
        );

        Ok(Self {
            _texture: texture,
            view,
            sampler,
            view_dimension,
        })
    }

    /// Returns a view into the texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns a sampler for the texture.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Creates the bind group layout entry for this texture, assigned to the
    /// given binding.
    pub fn create_texture_bind_group_layout_entry(
        &self,
        binding: u32,
    ) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: self.view_dimension,
                multisampled: false,
            },
            count: None,
        }
    }

    /// Creates the bind group layout entry for this texture's sampler, assigned
    /// to the given binding.
    pub fn create_sampler_bind_group_layout_entry(
        &self,
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
    fn create_empty_texture(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_size: wgpu::Extent3d,
        dimension: wgpu::TextureDimension,
        mip_level_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count,
            sample_count: 1,
            dimension,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[],
        })
    }

    fn write_data_to_texture(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        byte_buffer: &[u8],
        texel_description: TexelDescription,
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
                bytes_per_row: Some(texel_description.n_bytes() * texture_size.width),
                rows_per_image: Some(texture_size.height),
            },
            texture_size,
        );
    }

    fn create_view(
        texture: &wgpu::Texture,
        view_dimension: wgpu::TextureViewDimension,
    ) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(view_dimension),
            ..Default::default()
        })
    }

    fn create_sampler(
        device: &wgpu::Device,
        address_mode_u: wgpu::AddressMode,
        address_mode_v: wgpu::AddressMode,
        address_mode_w: wgpu::AddressMode,
        mag_filter: wgpu::FilterMode,
        min_filter: wgpu::FilterMode,
        mipmap_filter: wgpu::FilterMode,
        lod_min_clamp: f32,
        lod_max_clamp: f32,
        anisotropy_clamp: u16,
    ) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp,
            lod_max_clamp,
            anisotropy_clamp,
            ..Default::default()
        })
    }
}

impl TextureAddressingConfig {
    pub const CLAMPED: Self = Self {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
    };

    pub const REPEATING: Self = Self {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
    };
}

impl Default for TextureAddressingConfig {
    fn default() -> Self {
        Self::CLAMPED
    }
}

impl TextureFilteringConfig {
    pub const BASIC: Self = Self {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        max_mip_level_count: None,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: std::f32::MAX,
        anisotropy_clamp: 1,
    };

    pub const ANISOTROPIC_2X: Self = Self {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        max_mip_level_count: None,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: std::f32::MAX,
        anisotropy_clamp: 2,
    };

    pub const ANISOTROPIC_4X: Self = Self {
        anisotropy_clamp: 4,
        ..Self::ANISOTROPIC_2X
    };

    pub const ANISOTROPIC_8X: Self = Self {
        anisotropy_clamp: 8,
        ..Self::ANISOTROPIC_2X
    };

    pub const ANISOTROPIC_16X: Self = Self {
        anisotropy_clamp: 16,
        ..Self::ANISOTROPIC_2X
    };

    pub const LOOKUP: Self = Self {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        max_mip_level_count: None,
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: std::f32::MAX,
        anisotropy_clamp: 1,
    };
}

impl Default for TextureFilteringConfig {
    fn default() -> Self {
        Self::BASIC
    }
}

impl<T: TexelType> TextureLookupTable<T> {
    /// Wraps the lookup table with the given dimensions and data. The table is
    /// considered an array of 2D subtables if `depth_or_array_layers` is
    /// `ArrayLayers`, otherwise it is considered 1D if `depth_or_array_layers`
    /// and `height` are 1, 2D if only `depth_or_array_layers` is 1 and 3D
    /// otherwise. The lookup values in the `data` vector are assumed to be laid
    /// out in row-major order, with adjacent values varying in width first,
    /// then height and finally depth.
    ///
    /// # Panics
    /// - If the `data` vector is empty.
    /// - If the table shape is inconsistent with the number of data values.
    pub fn new(
        width: usize,
        height: usize,
        depth_or_array_layers: DepthOrArrayLayers,
        data: Vec<T>,
    ) -> Self {
        assert!(!data.is_empty(), "No data for lookup table");

        assert_eq!(
            width * height * (u32::from(depth_or_array_layers.unwrap()) as usize),
            data.len(),
            "Lookup table shape ({}, {}, {:?}) inconsistent with number of data values ({})",
            width,
            height,
            depth_or_array_layers,
            data.len()
        );

        let width = NonZeroU32::new(u32::try_from(width).unwrap()).unwrap();
        let height = NonZeroU32::new(u32::try_from(height).unwrap()).unwrap();

        Self {
            width,
            height,
            depth_or_array_layers,
            data,
        }
    }
}

impl<T: TexelType + Serialize + DeserializeOwned> TextureLookupTable<T> {
    /// Serializes the lookup table into the `MessagePack` format and saves it
    /// at the given path.
    pub fn save_to_file(&self, output_file_path: impl AsRef<Path>) -> Result<()> {
        let mut byte_buffer = Vec::new();
        self.serialize(&mut Serializer::new(&mut byte_buffer))?;
        scene::io::util::save_data_as_binary(output_file_path, &byte_buffer)?;
        Ok(())
    }

    /// Loads and returns the `MessagePack` serialized lookup table at the given
    /// path.
    pub fn read_from_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let table = from_read(reader)?;
        Ok(table)
    }
}

impl DepthOrArrayLayers {
    fn unwrap(&self) -> NonZeroU32 {
        match self {
            Self::Depth(depth) => *depth,
            Self::ArrayLayers(n_array_layers) => *n_array_layers,
        }
    }
}

impl MipmapGenerator {
    /// Creates a new mipmap generator
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../../shader/mipmap.wgsl"
            ))),
            label: Some("Mipmap shader"),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            label: Some("Mipmap sampler"),
            ..Default::default()
        });

        let pipelines_and_bind_group_layouts = TexelDescription::AVAILABLE_FORMATS
            .iter()
            .map(|&format| {
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "mainVS",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "mainFS",
                        targets: &[Some(format.into())],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    label: Some("Mipmap pipeline"),
                });

                // Get bind group layout determined from shader code
                let bind_group_layout = pipeline.get_bind_group_layout(0);

                (format, (pipeline, bind_group_layout))
            })
            .collect();

        Self {
            _shader: shader,
            sampler,
            pipelines_and_bind_group_layouts,
        }
    }

    fn generate_mipmaps(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
        mip_level_count: u32,
        label: &str,
    ) {
        let (pipeline, bind_group_layout) = self
            .pipelines_and_bind_group_layouts
            .get(&format)
            .expect("Tried to create mipmaps for unsupported format");

        let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mipmap command encoder"),
        });

        let texture_views: Vec<_> = (0..mip_level_count)
            .map(|mip_level| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip_level,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                    label: Some(&format!("{} mipmap view", label)),
                })
            })
            .collect();

        for target_mip_level in 1..mip_level_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: bind_group_layout,
                entries: &[
                    // Bind the view for the previous mip level
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &texture_views[target_mip_level - 1],
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: Some(&format!("{} mipmap bind group", label)),
            });

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                // Render to the view for the current mip level
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_views[target_mip_level],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some(&format!("{} mipmap render pass", label)),
            });

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);

            render_pass.draw(0..3, 0..1);
        }

        queue.submit(Some(command_encoder.finish()));
    }
}

/// Saves the texture at the given index of the given texture array as a color
/// or grayscale image at the given output path. The image file format is
/// automatically determined from the file extension.
///
/// # Errors
/// Returns an error if the format of the given texture is not supported.
pub fn save_texture_as_image_file<P: AsRef<Path>>(
    core_system: &CoreRenderingSystem,
    texture: &wgpu::Texture,
    texture_array_idx: u32,
    output_path: P,
) -> Result<()> {
    fn byte_to_float(byte: u8) -> f32 {
        f32::from(byte) / 255.0
    }

    fn float_to_byte(float: f32) -> u8 {
        (float.clamp(0.0, 1.0) * 255.0) as u8
    }

    fn gamma_corrected(linear_value: f32) -> f32 {
        f32::powf(linear_value, 0.4545) // ^(1 / 2.2)
    }

    fn gamma_corrected_depth(linear_value: f32) -> f32 {
        // To make small depths darker, we invert before gamma correcting, then
        // invert back
        1.0 - gamma_corrected(1.0 - linear_value)
    }

    fn gamma_corrected_byte(linear_value: u8) -> u8 {
        float_to_byte(gamma_corrected(byte_to_float(linear_value)))
    }

    let format = texture.format();

    match format {
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb
        | wgpu::TextureFormat::R8Unorm => {
            let mut data = extract_texture_data::<u8>(
                core_system.device(),
                core_system.queue(),
                texture,
                texture_array_idx,
            );

            if matches!(
                format,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
            ) {
                convert_bgra8_to_rgba8(&mut data);
            }

            if matches!(format, wgpu::TextureFormat::R8Unorm) {
                let mut image_buffer: ImageBuffer<Luma<u8>, _> =
                    ImageBuffer::from_raw(texture.width(), texture.height(), data).unwrap();

                for p in image_buffer.pixels_mut() {
                    p.0[0] = gamma_corrected_byte(p.0[0]);
                }

                let image_buffer: ImageBuffer<Luma<u16>, _> = image_buffer.convert();

                image_buffer.save(output_path)?;
            } else {
                let mut image_buffer =
                    ImageBuffer::<Rgba<u8>, _>::from_raw(texture.width(), texture.height(), data)
                        .unwrap();

                if matches!(
                    format,
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
                ) {
                    for p in image_buffer.pixels_mut() {
                        p.0[0] = gamma_corrected_byte(p.0[0]);
                        p.0[1] = gamma_corrected_byte(p.0[1]);
                        p.0[2] = gamma_corrected_byte(p.0[2]);
                    }
                }

                for p in image_buffer.pixels_mut() {
                    p.0[3] = 255;
                }

                image_buffer.save(output_path)?;
            }
        }
        wgpu::TextureFormat::Depth32Float | wgpu::TextureFormat::Rgba32Float => {
            let mut data = extract_texture_data::<f32>(
                core_system.device(),
                core_system.queue(),
                texture,
                texture_array_idx,
            );

            if matches!(format, wgpu::TextureFormat::Depth32Float) {
                for value in &mut data {
                    *value = gamma_corrected_depth(*value);
                }

                let image_buffer: ImageBuffer<Luma<f32>, _> =
                    ImageBuffer::from_raw(texture.width(), texture.height(), data).unwrap();

                let image_buffer: ImageBuffer<Luma<u16>, _> = image_buffer.convert();

                image_buffer.save(output_path)?;
            } else {
                for value in &mut data {
                    *value = gamma_corrected(*value);
                }

                let mut image_buffer: ImageBuffer<Rgba<f32>, _> =
                    ImageBuffer::from_raw(texture.width(), texture.height(), data).unwrap();

                for p in image_buffer.pixels_mut() {
                    p.0[3] = 1.0;
                }

                let image_buffer: ImageBuffer<Rgba<u8>, _> = image_buffer.convert();

                image_buffer.save(output_path)?;
            }
        }
        _ => {
            bail!(
                "Unsupported texture format for saving as image file: {:?}",
                format
            );
        }
    }

    Ok(())
}

fn extract_texture_data<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    texture_array_idx: u32,
) -> Vec<T> {
    assert!(texture_array_idx < texture.depth_or_array_layers());
    let texture_array_idx = texture_array_idx as usize;

    let width = texture.width();
    let height = texture.height();
    let texel_size = texture
        .format()
        .block_size(None)
        .expect("Texel block size unavailable");

    let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Texture copy encoder"),
    });

    let raw_buffer =
        vec![0; (texel_size * width * height * texture.depth_or_array_layers()) as usize];

    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: raw_buffer.as_slice(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("Texture buffer"),
    });

    command_encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(texel_size * width),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );

    queue.submit(std::iter::once(command_encoder.finish()));

    let buffer_slice = buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |result| result.unwrap());
    device.poll(wgpu::Maintain::Wait);
    let buffer_view = buffer_slice.get_mapped_range();

    // Extract only the data of the texture with the given texture array index
    let texture_image_size = (texel_size * width * height) as usize;
    let buffer_view = &buffer_view
        [texture_array_idx * texture_image_size..(texture_array_idx + 1) * texture_image_size];

    bytemuck::cast_slice(buffer_view).to_vec()
}

fn convert_bgra8_to_rgba8(bgra_bytes: &mut [u8]) {
    for bgra in bgra_bytes.chunks_exact_mut(4) {
        bgra.swap(0, 2);
    }
}
