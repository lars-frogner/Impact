//! Graphics rendering.

mod asset;
mod buffer;
mod camera;
mod core;
mod mesh;
mod render_pass;
mod tasks;
mod world;

pub use self::core::CoreRenderingSystem;
pub use asset::{Assets, ImageTexture, Shader};
pub use render_pass::{RenderPassRecorder, RenderPassSpecification};
pub use tasks::{Render, RenderingTag};
pub use world::{RenderData, SyncRenderData};

use crate::{geometry::GeometricalData, window::ControlFlow};
use anyhow::{Error, Result};
use std::sync::RwLock;

/// Container for all data and logic required for rendering.
#[derive(Debug)]
pub struct RenderingSystem {
    core_system: CoreRenderingSystem,
    assets: Assets,
    render_data: RwLock<RenderData>,
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
        let render_data = RwLock::new(RenderData::from_geometrical_data(
            &core_system,
            geometrical_data,
        ));

        let render_pass_recorders: Result<Vec<_>> = {
            let render_data_guard = render_data.read().unwrap();
            specifications
                .into_iter()
                .map(|template| {
                    RenderPassRecorder::new(
                        &core_system,
                        &assets,
                        render_data_guard.synchronized(),
                        template,
                    )
                })
                .collect()
        }; // <- Lock on `render_data` is released here

        Ok(Self {
            core_system,
            assets,
            render_data,
            render_passes,
            camera_id,
        })
    }

    /// Returns a reference to the core rendering system.
    pub fn core_system(&self) -> &CoreRenderingSystem {
        &self.core_system
    }

    /// Returns a reference to the [`RenderData`], guarded
    /// by a [`RwLock`].
    pub fn render_data(&self) -> &RwLock<RenderData> {
        &self.render_data
    }

    /// Creates and presents a rendering of the current data in the pipelines.
    ///
    /// # Errors
    /// Returns an error if:
    /// - If the surface texture to render to can not be obtained.
    /// - If recording a render pass fails.
    pub fn render(&self) -> Result<()> {
        let surface_texture = self.core_system.surface().get_current_texture()?;
        let view = Self::create_surface_texture_view(&surface_texture);

        let mut command_encoder = Self::create_render_command_encoder(self.core_system.device());

        {
            let render_data_guard = self.render_data.read().unwrap();
            for render_pass_recorder in &self.render_pass_recorders {
                render_pass_recorder.record_render_pass(
                    &self.assets,
                    render_data_guard.synchronized(),
                    &view,
                    &mut command_encoder,
                )?;
            }
        } // <- Lock on `self.render_data` is released here

        self.core_system
            .queue()
            .submit(std::iter::once(command_encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    /// Sets a new size for the rendering surface.
    pub fn resize_surface(&mut self, new_size: (u32, u32)) {
        self.core_system.resize_surface(new_size)
    }

    /// Initializes the surface for presentation using the
    /// current surface configuration.
    fn initialize_surface(&self) {
        self.core_system.initialize_surface()
    }

    fn handle_render_error(&self, error: Error, control_flow: &mut ControlFlow<'_>) {
        match error.downcast_ref() {
            // Recreate swap chain if lost
            Some(wgpu::SurfaceError::Lost) => self.initialize_surface(),
            // Quit if GPU is out of memory
            Some(wgpu::SurfaceError::OutOfMemory) => {
                control_flow.exit();
            }
            // Other errors should be resolved by the next frame, so we just log the error and continue
            _ => log::error!("{:?}", error),
        }
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
