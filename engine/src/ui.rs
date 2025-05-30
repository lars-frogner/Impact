//! User interface.

pub mod command;
pub mod window;

use crate::{
    application::Application, engine::Engine, game_loop::GameLoop,
    gpu::rendering::gui::GUIRenderingInput, window::Window,
};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};
use window::{UIEventHandlingResponse, UserInterfaceWindowIntegration};
use winit::event::{DeviceEvent, WindowEvent};

#[derive(Debug)]
pub struct UserInterface {
    app: Arc<dyn Application>,
    egui_ctx: egui::Context,
    window_integration: UserInterfaceWindowIntegration,
}

/// Configuration parameters for the user interface.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserInterfaceConfig {
    /// Whether the user interface should be visible as soon as the application
    /// starts.
    pub initially_visible: bool,
}

pub struct RawUserInterfaceOutput {
    output: egui::FullOutput,
}

#[derive(Clone, Debug, Default)]
pub struct UserInterfaceOutput {
    rendering_input: GUIRenderingInput,
}

impl UserInterface {
    pub fn new(app: Arc<dyn Application>, window: Window) -> Self {
        let egui_ctx = egui::Context::default();

        let window_integration = UserInterfaceWindowIntegration::new(window, egui_ctx.clone());

        Self {
            app,
            egui_ctx,
            window_integration,
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> UIEventHandlingResponse {
        self.window_integration.handle_window_event(event)
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        self.window_integration.handle_device_event(event);
    }

    pub fn run(&mut self, game_loop: &GameLoop, engine: &Engine) -> RawUserInterfaceOutput {
        let input = self.window_integration.take_raw_input();

        let mut output = self
            .egui_ctx
            .run(input, |ctx| self.app.run_ui(ctx, game_loop, engine));

        output = self.window_integration.handle_full_output(output);

        RawUserInterfaceOutput { output }
    }

    pub fn process_raw_output(&self, output: RawUserInterfaceOutput) -> UserInterfaceOutput {
        let RawUserInterfaceOutput { output } = output;

        let clipped_primitives = self
            .egui_ctx
            .tessellate(output.shapes, output.pixels_per_point);

        let rendering_input = GUIRenderingInput {
            textures_delta: output.textures_delta,
            clipped_primitives,
        };

        UserInterfaceOutput { rendering_input }
    }
}

impl Default for UserInterfaceConfig {
    fn default() -> Self {
        Self {
            initially_visible: true,
        }
    }
}

impl fmt::Debug for RawUserInterfaceOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawUserInterfaceOutput").finish()
    }
}

impl UserInterfaceOutput {
    pub fn rendering_input(&self) -> &GUIRenderingInput {
        &self.rendering_input
    }
}
