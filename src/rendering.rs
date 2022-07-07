//! Graphics rendering.

mod asset;
mod buffer;
mod camera;
mod core;
mod mesh;
mod render_pass;
mod world;

use crate::geometry::WorldData;
use anyhow::Result;
use winit::event::WindowEvent;

pub use self::core::CoreRenderingSystem;
pub use asset::{Assets, ImageTexture, Shader};
pub use render_pass::{RenderPassRecorder, RenderPassSpecification};
pub use world::RenderData;

/// Container for all data and logic required for rendering.
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
        world_data: &WorldData,
    ) -> Result<Self> {
        let render_data = RenderData::from_world_data(&core_system, world_data);

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

    /// Sets a new size for the rendering surface.
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        self.core_system.resize_surface(new_size)
    }

    /// Initializes the surface for presentation using the
    /// current surface configuration.
    pub fn initialize_surface(&mut self) {
        self.core_system.initialize_surface()
    }

    /// Updates any relevant state based on the given window event.
    ///
    /// # Returns
    /// A `bool` specifying whether the event should be handled further.
    pub fn handle_input_event(&mut self, window_event: &WindowEvent) -> bool {
        match window_event {
            // WindowEvent::CursorMoved { position, .. } => {
            //     let surface_config = self.core_system.surface_config();
            //     let r = (f64::from(position.x)) / (f64::from(surface_config.width));
            //     let g = (f64::from(position.y)) / (f64::from(surface_config.height));
            //     for pipeline in &mut self.pipelines {
            //         pipeline.set_clear_color(wgpu::Color {
            //             r,
            //             g,
            //             b: 0.0,
            //             a: 1.0,
            //         });
            //     }
            //     false
            // }
            _ => true,
        }
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
