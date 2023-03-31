//! Textures representing 2D images.

use crate::rendering::CoreRenderingSystem;
use anyhow::{anyhow, Result};
use image::{self, DynamicImage, GenericImageView};
use std::num::NonZeroU32;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use std::path::Path;
        use image::io::Reader as ImageReader;
    }
}

/// A texture representing a 2D image.
#[derive(Debug)]
pub struct ImageTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

/// A texture that can be used as a multisampled render target.
#[derive(Debug)]
pub struct MultisampledRenderTargetTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ColorType {
    Rgba,
    Grayscale,
}

impl ImageTexture {
    /// Creates a texture for the image file at the given path.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image file can not be read or decoded.
    /// - The image bytes can not be interpreted.
    /// - The image width or height is zero.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(
        core_system: &CoreRenderingSystem,
        image_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let image_path = image_path.as_ref();
        let image = ImageReader::open(image_path)?.decode()?;
        Self::from_image(core_system, image, &image_path.to_string_lossy())
    }

    /// Creates a texture for the image file represented by the given raw byte
    /// buffer.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image bytes can not be interpreted.
    /// - The image width or height is zero.
    pub fn from_bytes(
        core_system: &CoreRenderingSystem,
        byte_buffer: &[u8],
        label: &str,
    ) -> Result<Self> {
        let image = image::load_from_memory(byte_buffer)?;
        Self::from_image(core_system, image, label)
    }

    /// Creates a texture for the given loaded image.
    ///
    /// # Errors
    /// Returns an error if the image width or height is zero.
    pub fn from_image(
        core_system: &CoreRenderingSystem,
        image: DynamicImage,
        label: &str,
    ) -> Result<Self> {
        let (width, height) = image.dimensions();
        let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
        let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;

        Ok(if image.color().has_color() {
            Self::new(
                core_system,
                &image.into_rgba8(),
                ColorType::Rgba,
                width,
                height,
                label,
            )
        } else {
            Self::new(
                core_system,
                &image.into_luma8(),
                ColorType::Grayscale,
                width,
                height,
                label,
            )
        })
    }

    /// Creates a texture for the image represented by the given byte buffer,
    /// color type and dimensions.
    fn new(
        core_system: &CoreRenderingSystem,
        byte_buffer: &[u8],
        color_type: ColorType,
        width: NonZeroU32,
        height: NonZeroU32,
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

        let sampler = Self::create_sampler(device);

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

    fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
    }
}

impl MultisampledRenderTargetTexture {
    /// Creates a texture corresponding to the core system surface texture, but
    /// with the given sample count for multisampling.
    pub fn new(core_system: &CoreRenderingSystem, sample_count: u32) -> Self {
        let texture = Self::create_empty_surface_texture(
            core_system.device(),
            core_system.surface_config(),
            sample_count,
            "Multisampled surface texture",
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    /// Returns the multisampling sample count.
    pub fn sample_count(&self) -> u32 {
        self.texture.sample_count()
    }

    /// Returns a view into the texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Creates a new [`wgpu::Texture`] with the same dimensions and format as
    /// the specified surface, but with the given sample count for
    /// multisampling.
    fn create_empty_surface_texture(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
        sample_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some(label),
            view_formats: &[],
        })
    }
}

impl ColorType {
    fn n_bytes(&self) -> u32 {
        match self {
            Self::Rgba => 4,
            Self::Grayscale => 1,
        }
    }

    fn texture_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Grayscale => wgpu::TextureFormat::R8Unorm,
        }
    }
}
