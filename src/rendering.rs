//! Graphics rendering.

mod asset;
mod buffer;
mod camera;
mod core;
mod mesh;
mod render_pass;
mod world;

pub use self::core::CoreRenderingSystem;
pub use asset::{Assets, ImageTexture, Shader};
pub use render_pass::{RenderPassRecorder, RenderPassSpecification};
pub use world::RenderData;

use crate::geometry::GeometricalData;
use anyhow::Result;

/// Container for all data and logic required for rendering.
#[derive(Debug)]
pub struct RenderingSystem {
    core_system: CoreRenderingSystem,
    assets: Assets,
    render_data: RenderData,
    render_pass_recorders: Vec<RenderPassRecorder>,
}

impl RenderingSystem {
    /// Creates a new rendering system consisting of the given
    /// core system and rendering pipelines.
    pub async fn new(
        core_system: CoreRenderingSystem,
        assets: Assets,
        specifications: Vec<RenderPassSpecification>,
        geometrical_data: &GeometricalData,
    ) -> Result<Self> {
        let render_data = RenderData::from_geometrical_data(&core_system, geometrical_data);

        let render_pass_recorders: Result<Vec<_>> = specifications
            .into_iter()
            .map(|template| RenderPassRecorder::new(&core_system, &assets, &render_data, template))
            .collect();

        Ok(Self {
            core_system,
            assets,
            render_data,
            render_pass_recorders: render_pass_recorders?,
        })
    }

    /// Creates and presents a rendering of the current data in the pipelines.
    ///
    /// # Errors
    /// Returns an error if:
    /// - If the surface texture to render to can not be obtained.
    /// - If recording a render pass fails.
    pub fn render(&mut self) -> Result<()> {
        let surface_texture = self.core_system.surface().get_current_texture()?;
        let view = Self::create_surface_texture_view(&surface_texture);

        let mut command_encoder = Self::create_render_command_encoder(self.core_system.device());

        for render_pass_recorder in &mut self.render_pass_recorders {
            render_pass_recorder.record_render_pass(
                &self.assets,
                &self.render_data,
                &view,
                &mut command_encoder,
            )?;
        }

        self.core_system
            .queue()
            .submit(std::iter::once(command_encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    /// Performs any required updates for keeping the render data in
    /// sync with the geometrical data.
    ///
    /// # Notes
    /// - Render data entries for which the associated geometrical data no
    /// longer exists will be removed.
    /// - Mutable access to the geometrical data is required in order to reset
    /// all change trackers.
    pub fn sync_with_geometry(&mut self, geometrical_data: &mut GeometricalData) {
        self.render_data
            .sync_with_geometry(&self.core_system, geometrical_data);
    }

    /// Sets a new size for the rendering surface.
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        self.core_system.resize_surface(new_size)
    }

    /// Initializes the surface for presentation using the
    /// current surface configuration.
    pub fn initialize_surface(&mut self) {
        self.core_system.initialize_surface()
    }

    fn create_surface_texture_view(surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_render_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render encoder"),
        })
    }
}
