//! Graphics rendering.

mod buffer;
mod camera;
mod core;
mod pipeline;
mod shader;
mod texture;

use anyhow::Result;
use winit::event::WindowEvent;

pub use self::core::CoreRenderingSystem;
pub use buffer::{BufferableIndex, BufferableVertex, IndexBuffer, VertexBuffer};
pub use pipeline::{RenderingPipeline, RenderingPipelineBuilder};
pub use shader::Shader;
pub use texture::ImageTexture;

pub struct RenderingSystem {
    core_system: CoreRenderingSystem,
    pipelines: Vec<RenderingPipeline>,
}

impl RenderingSystem {
    /// Creates a new rendering system consisting of the given
    /// core system and rendering pipelines.
    pub async fn new(core_system: CoreRenderingSystem, pipelines: Vec<RenderingPipeline>) -> Self {
        Self {
            core_system,
            pipelines,
        }
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
            WindowEvent::CursorMoved { position, .. } => {
                let surface_config = self.core_system.surface_config();
                let r = (position.x as f64) / (surface_config.width as f64);
                let g = (position.y as f64) / (surface_config.height as f64);
                for pipeline in &mut self.pipelines {
                    pipeline.set_clear_color(wgpu::Color {
                        r,
                        g,
                        b: 0.0,
                        a: 1.0,
                    });
                }
                false
            }
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

        for pipeline in &mut self.pipelines {
            pipeline.record_render_passes(&view, &mut command_encoder)?;
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
