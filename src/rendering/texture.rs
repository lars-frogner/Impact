//! Textures.

mod attachment;
mod shadow_map;

pub use attachment::{
    DepthTexture, MultisampledSurfaceTexture, RenderAttachmentQuantity,
    RenderAttachmentQuantitySet, RenderAttachmentTextureManager, RENDER_ATTACHMENT_BINDINGS,
    RENDER_ATTACHMENT_FLAGS, RENDER_ATTACHMENT_FORMATS,
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
use std::{fs::File, io::BufReader, num::NonZeroU32, path::Path};
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

/// Configuration for [`Texture`]s.
#[derive(Clone, Debug, Default)]
pub struct TextureConfig {
    /// The color space that the texel data values should be assumed to be
    /// stored in.
    pub color_space: ColorSpace,
    /// How addressing outside the [0, 1] range for the U texture coordinate
    /// should be handled.
    pub address_mode_u: wgpu::AddressMode,
    /// How addressing outside the [0, 1] range for the V texture coordinate
    /// should be handled.
    pub address_mode_v: wgpu::AddressMode,
    /// How addressing outside the [0, 1] range for the W texture coordinate
    /// should be handled.
    pub address_mode_w: wgpu::AddressMode,
    /// How to filter the texture when it needs to be magnified.
    pub mag_filter: wgpu::FilterMode,
    /// How to filter the texture when it needs to be minified.
    pub min_filter: wgpu::FilterMode,
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

impl Default for ColorSpace {
    fn default() -> Self {
        Self::Linear
    }
}

impl TexelDescription {
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
    /// configuration parameters.
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
        image_path: impl AsRef<Path>,
        config: TextureConfig,
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
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_bytes(
        core_system: &CoreRenderingSystem,
        byte_buffer: &[u8],
        config: TextureConfig,
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
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for for copying
    ///   data between buffers and textures).
    /// - The image is grayscale and the color space in the configuration is not
    ///   linear.
    pub fn from_image(
        core_system: &CoreRenderingSystem,
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
                &image.into_rgba8(),
                width,
                height,
                DepthOrArrayLayers::Depth(depth),
                TexelDescription::Rgba8(config.color_space),
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
                DepthOrArrayLayers::Depth(depth),
                TexelDescription::Grayscale8,
                config,
                label,
            )
        }
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
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
        };

        Self::new(
            core_system,
            byte_buffer,
            table.width,
            table.height,
            table.depth_or_array_layers,
            T::DESCRIPTION,
            config,
            label,
        )
    }

    /// Creates a texture for the data contained in the given byte buffer, with
    /// the given dimensions and texel description, using the given
    /// configuration parameters.
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
        byte_buffer: &[u8],
        width: NonZeroU32,
        height: NonZeroU32,
        depth_or_array_layers: DepthOrArrayLayers,
        texel_description: TexelDescription,
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

        let (dimension, view_dimension) = if depth_or_array_layers.is_array_layers() {
            (
                wgpu::TextureDimension::D2,
                wgpu::TextureViewDimension::D2Array,
            )
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

        let texture =
            Self::create_empty_texture(device, texel_description, texture_size, dimension, label);

        Self::write_data_to_texture(
            core_system.queue(),
            &texture,
            byte_buffer,
            texel_description,
            texture_size,
        );

        let view = Self::create_view(&texture, view_dimension);

        let sampler = Self::create_sampler(
            device,
            config.address_mode_u,
            config.address_mode_v,
            config.address_mode_w,
            config.mag_filter,
            config.min_filter,
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
        texel_description: TexelDescription,
        texture_size: wgpu::Extent3d,
        dimension: wgpu::TextureDimension,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format: texel_description.texture_format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
                bytes_per_row: Some(
                    NonZeroU32::new(texel_description.n_bytes() * texture_size.width).unwrap(),
                ),
                rows_per_image: Some(NonZeroU32::new(texture_size.height).unwrap()),
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
    ) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
    }
}

impl TextureConfig {
    pub const REPEATING_COLOR_TEXTRUE: Self = Self {
        color_space: ColorSpace::Srgb,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
    };

    pub const REPEATING_NON_COLOR_TEXTRUE: Self = Self {
        color_space: ColorSpace::Linear,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
    };
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
    pub fn save_to_file(self, output_file_path: impl AsRef<Path>) -> Result<Self> {
        let mut byte_buffer = Vec::new();
        self.serialize(&mut Serializer::new(&mut byte_buffer))?;
        scene::io::util::save_data_as_binary(output_file_path, &byte_buffer)?;
        Ok(self)
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
    fn is_array_layers(&self) -> bool {
        matches!(self, Self::ArrayLayers(_))
    }

    fn unwrap(&self) -> NonZeroU32 {
        match self {
            Self::Depth(depth) => *depth,
            Self::ArrayLayers(n_array_layers) => *n_array_layers,
        }
    }
}

/// Saves the given color texture as a color image at the given output path. The
/// image file format is automatically determined from the file extension.
///
/// Supported texture formats are RGBA8 and BGRA8.
///
/// # Errors
/// Returns an error if the format of the given texture is not supported.
///
/// # Panics
/// If the texture is a texture array with multiple textures.
pub fn save_color_texture_as_image_file<P: AsRef<Path>>(
    core_system: &CoreRenderingSystem,
    texture: &wgpu::Texture,
    output_path: P,
) -> Result<()> {
    assert_eq!(texture.depth_or_array_layers(), 1);

    let mut data = extract_texture_data(core_system.device(), core_system.queue(), texture, 0);

    match texture.format() {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {}
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
            convert_bgra8_to_rgba8(&mut data);
        }
        format => {
            bail!(
                "Unsupported texture format for saving as color image file: {:?}",
                format
            );
        }
    }

    let image_buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(texture.width(), texture.height(), data).unwrap();

    image_buffer.save(output_path)?;

    Ok(())
}

/// Saves the texture at the given index of the given depth texture array as a
/// grayscale image at the given output path. The image file format is
/// automatically determined from the file extension.
///
/// The supported texture format is [`wgpu::TextureFormat::Depth32Float`].
///
/// # Errors
/// Returns an error if the format of the given texture is not supported.
pub fn save_depth_texture_as_image_file<P: AsRef<Path>>(
    core_system: &CoreRenderingSystem,
    texture: &wgpu::Texture,
    texture_array_idx: u32,
    output_path: P,
) -> Result<()> {
    if texture.format() != wgpu::TextureFormat::Depth32Float {
        bail!(
            "Unsupported depth texture format for saving as image file: {:?}",
            texture.format()
        );
    }

    let mut data = extract_texture_data::<f32>(
        core_system.device(),
        core_system.queue(),
        texture,
        texture_array_idx,
    );

    // Gamma correction
    for value in &mut data {
        *value = f32::powf(*value, 2.2);
    }

    let image_buffer: ImageBuffer<Luma<f32>, _> =
        ImageBuffer::from_raw(texture.width(), texture.height(), data).unwrap();

    let image_buffer: ImageBuffer<Luma<u16>, _> = image_buffer.convert();

    image_buffer.save(output_path)?;

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
    let texel_size = u32::from(texture.format().describe().block_size);

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
                bytes_per_row: Some(NonZeroU32::new(texel_size * width).unwrap()),
                rows_per_image: Some(NonZeroU32::new(height).unwrap()),
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
