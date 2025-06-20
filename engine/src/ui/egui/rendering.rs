//! Rendering of [`egui`] based user interfaces.

use crate::gpu::{
    GraphicsDevice,
    query::TimestampQueryRegistry,
    rendering::{render_command, surface::RenderingSurface},
};
use core::fmt;
use std::borrow::Cow;

#[allow(missing_debug_implementations)]
pub struct EguiRenderer {
    renderer: egui_wgpu::Renderer,
}

#[derive(Clone, Debug, Default)]
pub struct EguiRenderingInput {
    pub textures_delta: egui::TexturesDelta,
    pub clipped_primitives: Vec<egui::ClippedPrimitive>,
}

impl EguiRenderer {
    pub fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        dithering: bool,
    ) -> Self {
        let renderer = egui_wgpu::Renderer::new(
            graphics_device.device(),
            rendering_surface.texture_format(),
            None,
            1,
            dithering,
        );
        Self { renderer }
    }

    pub fn update_resources_and_record_render_pass(
        &mut self,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        input: &EguiRenderingInput,
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

    fn screen_descriptor(rendering_surface: &RenderingSurface) -> egui_wgpu::ScreenDescriptor {
        let (width, height) = rendering_surface.surface_dimensions();
        let pixels_per_point = rendering_surface.pixels_per_point();
        egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width.get(), height.get()],
            pixels_per_point: pixels_per_point as f32,
        }
    }
}

impl fmt::Debug for EguiRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GUIRenderer").finish()
    }
}
