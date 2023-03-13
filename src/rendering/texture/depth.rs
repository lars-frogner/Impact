//! Textures storing fragment depths.

use crate::rendering::CoreRenderingSystem;
use anyhow::Result;
use std::path::Path;

/// Texture for storing fragment depths.
#[derive(Debug)]
pub struct DepthTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl DepthTexture {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// Creates a new depth texture of the same size as the rendering surface in
    /// `core_system` and with the given sample count for multisampling.
    pub fn new(core_system: &CoreRenderingSystem, sample_count: u32) -> Self {
        let device = core_system.device();
        let surface_config = core_system.surface_config();

        let texture_size = wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };

        let texture =
            Self::create_empty_depth32_texture(device, texture_size, sample_count, "Depth texture");

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    /// Returns a view into the depth texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Saves the texture as a grayscale image at the given output path. The
    /// image file format is automatically determined from the file extension.
    pub fn save_as_image_file<P: AsRef<Path>>(
        &self,
        core_system: &CoreRenderingSystem,
        output_path: P,
    ) -> Result<()> {
        super::save_depth_texture_as_image_file(core_system, &self.texture, 0, output_path)
    }

    /// Creates a new [`wgpu::Texture`] configured to hold 2D depth data
    /// in 32-bit float format.
    fn create_empty_depth32_texture(
        device: &wgpu::Device,
        texture_size: wgpu::Extent3d,
        sample_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: Some(label),
            view_formats: &[],
        })
    }
}
