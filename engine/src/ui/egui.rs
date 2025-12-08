//! Integration of user interfaces based on [`egui`].

mod rendering;
mod window;

use crate::{
    application::Application, engine::Engine, lock_order::OrderedRwLock, ui::UserInterface,
    window::Window,
};
use anyhow::Result;
use impact_alloc::arena::Arena;
use impact_gpu::{device::GraphicsDevice, timestamp_query::TimestampQueryRegistry, wgpu};
use impact_rendering::surface::RenderingSurface;
use parking_lot::Mutex;
use rendering::{EguiRenderer, EguiRenderingInput};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use window::EguiWindowIntegration;

/// Coordinator between an [`egui`] user interface implemented in the
/// [`Application`] and the engine's window and rendering systems.
#[derive(Debug)]
pub struct EguiUserInterface {
    app: Arc<dyn Application>,
    egui_ctx: egui::Context,
    window: Window,
    window_integration: Mutex<EguiWindowIntegration>,
    renderer: Mutex<EguiRenderer>,
    rendering_input: Mutex<Option<EguiRenderingInput>>,
}

/// Configuration options for [`egui`] based user interfaces.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EguiUserInterfaceConfig {
    pub dithering: bool,
}

impl EguiUserInterface {
    pub fn new(
        config: EguiUserInterfaceConfig,
        app: Arc<dyn Application>,
        engine: &Engine,
        window: Window,
    ) -> Self {
        let egui_ctx = egui::Context::default();

        let window_integration = EguiWindowIntegration::new(egui_ctx.clone(), &window);

        let rendering_system = engine.renderer().oread();
        let renderer = EguiRenderer::new(
            rendering_system.graphics_device(),
            rendering_system.rendering_surface(),
            config.dithering,
        );

        Self {
            app,
            window,
            egui_ctx,
            window_integration: Mutex::new(window_integration),
            renderer: Mutex::new(renderer),
            rendering_input: Mutex::new(None),
        }
    }

    fn process_raw_output(
        egui_ctx: &egui::Context,
        output: egui::FullOutput,
    ) -> EguiRenderingInput {
        let clipped_primitives = egui_ctx.tessellate(output.shapes, output.pixels_per_point);

        EguiRenderingInput {
            textures_delta: output.textures_delta,
            clipped_primitives,
        }
    }
}

impl UserInterface for EguiUserInterface {
    fn process(&self, arena: &Arena, engine: &Engine) -> Result<()> {
        let input = self
            .window_integration
            .lock()
            .take_raw_input(&self.egui_ctx, &self.window);

        let mut output = self.app.run_egui_ui(arena, &self.egui_ctx, input, engine);

        output =
            self.window_integration
                .lock()
                .handle_full_output(&self.egui_ctx, &self.window, output);

        // TODO: Output processing could be split into a separate task for
        // performance
        let rendering_input = Self::process_raw_output(&self.egui_ctx, output);
        *self.rendering_input.lock() = Some(rendering_input);

        Ok(())
    }

    fn render(
        &self,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        let Some(rendering_input) = self.rendering_input.lock().take() else {
            return Ok(());
        };

        self.renderer
            .lock()
            .update_resources_and_record_render_pass(
                graphics_device,
                rendering_surface,
                surface_texture_view,
                &rendering_input,
                timestamp_recorder,
                command_encoder,
            );

        Ok(())
    }
}

impl Default for EguiUserInterfaceConfig {
    fn default() -> Self {
        Self { dithering: false }
    }
}

/// Hide the cursor in the `egui` output.
pub fn ensure_cursor_hidden(output: &mut egui::FullOutput) {
    output.platform_output.cursor_icon = egui::CursorIcon::None;
}

/// Adds a viewport command in the `egui` output for confining the cursor to the
/// window bounds.
pub fn confine_cursor(output: &mut egui::FullOutput) {
    append_viewport_commands(
        output,
        [egui::ViewportCommand::CursorGrab(
            egui::CursorGrab::Confined,
        )],
    );
}

/// Adds a viewport command in the `egui` output for release cursor confinement.
pub fn unconfine_cursor(output: &mut egui::FullOutput) {
    append_viewport_commands(
        output,
        [egui::ViewportCommand::CursorGrab(egui::CursorGrab::None)],
    );
}

/// Helper function to append viewport commands to the `egui` output.
pub fn append_viewport_commands(
    output: &mut egui::FullOutput,
    commands: impl IntoIterator<Item = egui::ViewportCommand>,
) {
    if let Some(viewport_output) = output.viewport_output.get_mut(&egui::ViewportId::ROOT) {
        viewport_output.commands.extend(commands);
    }
}
