//! Textures representing 2D images.

use super::super::CoreRenderingSystem;
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
pub struct ImageTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group_label: String,
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
    pub fn from_path<P: AsRef<Path>>(
        core_system: &CoreRenderingSystem,
        image_path: P,
        label: &str,
    ) -> Result<Self> {
        let image = ImageReader::open(image_path)?.decode()?;
        Self::from_image(core_system, &image, label)
    }

    /// Creates a texture for the image file represented by the given
    /// raw byte buffer.
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
        Self::from_image(core_system, &image, label)
    }

    /// Creates a texture for the given loaded image.
    ///
    /// # Errors
    /// Returns an error if the image width or height is zero.
    pub fn from_image(
        core_system: &CoreRenderingSystem,
        image: &DynamicImage,
        label: &str,
    ) -> Result<Self> {
        let (width, height) = image.dimensions();
        Ok(Self::from_rgba_bytes(
            core_system,
            &image.to_rgba8(),
            (
                NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?,
                NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?,
            ),
            label,
        ))
    }

    /// Creates a texture for the image represented by the given
    /// RGBA byte buffer and dimensions.
    fn from_rgba_bytes(
        core_system: &CoreRenderingSystem,
        rgba_buffer: &[u8],
        (width, height): (NonZeroU32, NonZeroU32),
        label: &str,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: u32::from(width),
            height: u32::from(height),
            depth_or_array_layers: 1,
        };

        let texture = Self::create_empty_rgba8_texture(core_system.device(), texture_size, label);
        Self::write_data_to_texture(core_system.queue(), &texture, rgba_buffer, texture_size);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = Self::create_sampler(core_system.device());

        Self {
            _texture: texture,
            view,
            sampler,
            bind_group_label: format!("{} bind group", label),
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

    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // The sampler binding type must be consistent with the `filterable`
                    // field in the texture sample type.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Image texture bind group layout"),
        })
    }

    pub fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture: &ImageTexture,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(texture.sampler()),
                },
            ],
            label: Some(texture.bind_group_label()),
        })
    }

    fn bind_group_label(&self) -> &str {
        self.bind_group_label.as_str()
    }

    /// Creates a new `wgpu::Texture` configured to hold 2D image data
    /// in RGBA8 format.
    fn create_empty_rgba8_texture(
        device: &wgpu::Device,
        texture_size: wgpu::Extent3d,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
        })
    }

    fn write_data_to_texture(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        rgba_buffer: &[u8],
        texture_size: wgpu::Extent3d,
    ) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * texture_size.width),
                rows_per_image: NonZeroU32::new(texture_size.height),
            },
            texture_size,
        )
    }

    fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })
    }
}
