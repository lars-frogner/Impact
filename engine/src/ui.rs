//! Abstractions and helpers for UI systems.

#[cfg(feature = "egui")]
pub mod egui;
#[cfg(feature = "window")]
pub mod window;

use anyhow::Result;
use impact_gpu::{device::GraphicsDevice, timestamp_query::TimestampQueryRegistry, wgpu};
use impact_rendering::surface::RenderingSurface;

/// Core trait that all user interface implementations must implement.
pub trait UserInterface: Send + Sync + std::fmt::Debug {
    /// Handle UI logic and process and store output.
    ///
    /// This is called once at the beginning of each frame.
    fn process(&self) -> Result<()>;

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

/// No-op implementation of the [`UserInterface`] trait.
#[derive(Clone, Copy, Debug)]
pub struct NoUserInterface;

impl UserInterface for NoUserInterface {
    fn process(&self) -> Result<()> {
        Ok(())
    }

    fn render(
        &self,
        _graphics_device: &GraphicsDevice,
        _rendering_surface: &RenderingSurface,
        _surface_texture_view: &wgpu::TextureView,
        _timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        _command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        Ok(())
    }
}
