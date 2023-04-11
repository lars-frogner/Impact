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

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, bail, Result};
use bytemuck::Pod;
use image::{
    self, buffer::ConvertBuffer, io::Reader as ImageReader, DynamicImage, GenericImageView,
    ImageBuffer, Luma, Rgba,
};
use std::{num::NonZeroU32, path::Path};
use wgpu::util::DeviceExt;

/// A texture holding multidimensional data.
#[derive(Debug)]
pub struct Texture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

/// Configuration for [`Texture`]s.
#[derive(Clone, Debug, Default)]
pub struct TextureConfig {
    /// The color space that the pixel values should be assumed to be stored in.
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

impl Texture {
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

        Ok(if image.color().has_color() {
            Self::new(
                core_system,
                &image.into_rgba8(),
                width,
                height,
                depth,
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
                depth,
                ColorType::Grayscale8,
                config,
                label,
            )
        })
    }

    /// Creates a texture for the data contained in the given byte buffer, with
    /// the given dimensions and color type, using the given configuration
    /// parameters.
    fn new(
        core_system: &CoreRenderingSystem,
        byte_buffer: &[u8],
        width: NonZeroU32,
        height: NonZeroU32,
        depth: NonZeroU32,
        color_type: ColorType,
        config: TextureConfig,
        label: &str,
    ) -> Self {
        let device = core_system.device();

        let texture_size = wgpu::Extent3d {
            width: u32::from(width),
            height: u32::from(height),
            depth_or_array_layers: u32::from(depth),
        };

        let dimension = if texture_size.depth_or_array_layers > 1 {
            wgpu::TextureDimension::D3
        } else {
            wgpu::TextureDimension::D2
        };

        let texture =
            Self::create_empty_texture(device, color_type, texture_size, dimension, label);

        Self::write_data_to_texture(
            core_system.queue(),
            &texture,
            byte_buffer,
            color_type,
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = Self::create_sampler(
            device,
            config.address_mode_u,
            config.address_mode_v,
            config.address_mode_w,
            config.mag_filter,
            config.min_filter,
        );

        Self {
            texture,
            view,
            sampler,
        }
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
                view_dimension: if self.texture.depth_or_array_layers() > 1 {
                    wgpu::TextureViewDimension::D3
                } else {
                    wgpu::TextureViewDimension::D2
                },
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
        color_type: ColorType,
        texture_size: wgpu::Extent3d,
        dimension: wgpu::TextureDimension,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format: color_type.texture_format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[],
        })
    }

    fn write_data_to_texture(
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
                bytes_per_row: Some(
                    NonZeroU32::new(color_type.n_bytes() * texture_size.width).unwrap(),
                ),
                rows_per_image: Some(NonZeroU32::new(texture_size.height).unwrap()),
            },
            texture_size,
        );
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

impl Default for ColorSpace {
    fn default() -> Self {
        Self::Linear
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
    let pixel_size = u32::from(texture.format().describe().block_size);

    let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Texture copy encoder"),
    });

    let raw_buffer =
        vec![0; (pixel_size * width * height * texture.depth_or_array_layers()) as usize];

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
                bytes_per_row: Some(NonZeroU32::new(pixel_size * width).unwrap()),
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
    let texture_image_size = (pixel_size * width * height) as usize;
    let buffer_view = &buffer_view
        [texture_array_idx * texture_image_size..(texture_array_idx + 1) * texture_image_size];

    bytemuck::cast_slice(buffer_view).to_vec()
}

fn convert_bgra8_to_rgba8(bgra_bytes: &mut [u8]) {
    for bgra in bgra_bytes.chunks_exact_mut(4) {
        bgra.swap(0, 2);
    }
}
