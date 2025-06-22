//! Abstractions and helpers for UI systems.

pub mod tasks;

#[cfg(feature = "egui")]
pub mod egui;
#[cfg(feature = "window")]
pub mod window;

use crate::{engine::Engine, gpu::rendering::surface::RenderingSurface};
use anyhow::Result;
use impact_gpu::{device::GraphicsDevice, query::TimestampQueryRegistry};

/// Core trait that all user interface implementations must implement.
pub trait UserInterface: Send + Sync + std::fmt::Debug {
    /// Handle UI logic and process and store output.
    ///
    /// This is called once at the beginning of each frame.
    fn process(&self, engine: &Engine) -> Result<()>;

    /// Render the output from [`Self::process`].
    fn render(
        &self,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()>;
}
