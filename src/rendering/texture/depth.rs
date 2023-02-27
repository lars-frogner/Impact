//! Textures storing fragment depths.

use crate::rendering::CoreRenderingSystem;
use anyhow::Result;
use std::path::Path;

/// Texture for storing fragment depths.
#[derive(Debug)]
pub struct DepthTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl DepthTexture {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// Creates a new depth texture of the same size as the rendering surface in
    /// `core_system`.
    pub fn new(core_system: &CoreRenderingSystem, label: &str) -> Self {
        let device = core_system.device();
        let surface_config = core_system.surface_config();

        let texture_size = wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };

        let texture = Self::create_empty_depth32_texture(device, texture_size, label);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = Self::create_sampler(device);

        Self {
            texture,
            view,
            sampler,
        }
    }

    /// Returns a view into the depth texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns a sampler for the depth texture.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Saves the texture as a grayscale image at the given output path. The
    /// image file format is automatically determined from the file extension.
    pub fn save_as_image_file<P: AsRef<Path>>(
        &self,
        core_system: &CoreRenderingSystem,
        output_path: P,
    ) -> Result<()> {
        super::save_depth_texture_as_image_file(core_system, &self.texture, output_path)
    }

    /// Creates a new [`wgpu::Texture`] configured to hold 2D depth data
    /// in 32-bit float format.
    fn create_empty_depth32_texture(
        device: &wgpu::Device,
        texture_size: wgpu::Extent3d,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            label: Some(label),
            view_formats: &[],
        })
    }

    fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        })
    }
}
