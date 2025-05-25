//! Rendering of the graphical user interface.

use crate::gpu::{
    GraphicsDevice,
    query::TimestampQueryRegistry,
    rendering::{render_command, surface::RenderingSurface},
};
use core::fmt;
use egui_wgpu::{Renderer, ScreenDescriptor};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Manager of commands for rendering the graphical user interface.
#[allow(missing_debug_implementations)]
pub struct GUIRenderer {
    renderer: Renderer,
}

/// Configuration options for GUI rendering.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GUIRenderingConfig {
    pub dithering: bool,
}

impl GUIRenderer {
    /// Creates a new GUI renderer with the given configuration options.
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        config: &GUIRenderingConfig,
    ) -> Self {
        let renderer = Renderer::new(
            graphics_device.device(),
            rendering_surface.texture_format(),
            None,
            1,
            config.dithering,
        );
        Self { renderer }
    }

    /// Records all GUI render commands into the given command encoder.
    pub fn record_commands(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) {
        let color_attachments = [Some(Self::color_attachment(surface_texture_view))];

        let mut render_pass = render_command::begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &color_attachments,
            None,
            Cow::Borrowed("GUI render pass"),
        )
        .forget_lifetime();

        let screen_descriptor = Self::screen_descriptor(rendering_surface);

        self.renderer
            .render(&mut render_pass, &[], &screen_descriptor);

        log::trace!("Recorded GUI render pass");
    }

    fn color_attachment(
        surface_texture_view: &wgpu::TextureView,
    ) -> wgpu::RenderPassColorAttachment<'_> {
        wgpu::RenderPassColorAttachment {
            view: surface_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        }
    }

    fn screen_descriptor(rendering_surface: &RenderingSurface) -> ScreenDescriptor {
        let (width, height) = rendering_surface.surface_dimensions();
        let pixels_per_point = rendering_surface.pixels_per_point();
        ScreenDescriptor {
            size_in_pixels: [width.get(), height.get()],
            pixels_per_point: pixels_per_point as f32,
        }
    }
}

impl fmt::Debug for GUIRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GUIRenderer").finish()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for GUIRenderingConfig {
    fn default() -> Self {
        Self { dithering: false }
    }
}
