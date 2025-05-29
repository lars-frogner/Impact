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
    _config: GUIRenderingConfig,
}

/// Configuration options for GUI rendering.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GUIRenderingConfig {
    pub dithering: bool,
}

#[derive(Clone, Debug, Default)]
pub struct GUIRenderingInput {
    pub textures_delta: egui::TexturesDelta,
    pub clipped_primitives: Vec<egui::ClippedPrimitive>,
}

impl GUIRenderer {
    /// Creates a new GUI renderer with the given configuration options.
    pub fn new(
        config: GUIRenderingConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
    ) -> Self {
        let renderer = Renderer::new(
            graphics_device.device(),
            rendering_surface.texture_format(),
            None,
            1,
            config.dithering,
        );
        Self {
            renderer,
            _config: config,
        }
    }

    pub fn update_resources_and_record_render_pass(
        &mut self,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        input: &GUIRenderingInput,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) {
        let device = graphics_device.device();
        let queue = graphics_device.queue();

        let screen_descriptor = Self::screen_descriptor(rendering_surface);

        for (texture_id, image_delta) in &input.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *texture_id, image_delta);
        }

        self.renderer.update_buffers(
            device,
            queue,
            command_encoder,
            &input.clipped_primitives,
            &screen_descriptor,
        );

        let color_attachment = Self::color_attachment(surface_texture_view);

        let mut render_pass = render_command::begin_single_render_pass(
            command_encoder,
            timestamp_recorder,
            &[Some(color_attachment)],
            None,
            Cow::Borrowed("GUI render pass"),
        )
        .forget_lifetime();

        self.renderer.render(
            &mut render_pass,
            &input.clipped_primitives,
            &screen_descriptor,
        );

        for texture_id in &input.textures_delta.free {
            self.renderer.free_texture(texture_id);
        }

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
