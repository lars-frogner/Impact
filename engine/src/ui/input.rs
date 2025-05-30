//! Input management for user interface.

use crate::window::Window;
use std::fmt;
use winit::event::{DeviceEvent, WindowEvent};

pub struct UserInterfaceInputManager {
    window: Window,
    egui_ctx: egui::Context,
    state: egui_winit::State,
}

#[derive(Clone, Debug)]
pub struct UIEventHandlingResponse {
    pub event_consumed: bool,
}

impl UserInterfaceInputManager {
    pub fn new(window: Window, egui_ctx: egui::Context) -> Self {
        let state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.window(),
            Some(window.pixels_per_point() as f32),
            window.window().theme(),
            None,
        );
        Self {
            window,
            egui_ctx,
            state,
        }
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> UIEventHandlingResponse {
        let egui_winit::EventResponse {
            consumed,
            repaint: _, // We always repaint
        } = self.state.on_window_event(self.window.window(), event);

        UIEventHandlingResponse {
            event_consumed: consumed,
        }
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.state.on_mouse_motion(*delta);
        }
    }

    pub fn take_raw_input(&mut self) -> egui::RawInput {
        let input = self.state.egui_input_mut();
        let viewport_info = input.viewports.entry(egui::ViewportId::ROOT).or_default();

        egui_winit::update_viewport_info(
            viewport_info,
            &self.egui_ctx,
            self.window.window(),
            false,
        );

        self.state.take_egui_input(self.window.window())
    }

    pub fn handle_output(&mut self, mut output: egui::FullOutput) -> egui::FullOutput {
        self.state
            .handle_platform_output(self.window.window(), output.platform_output.take());

        if let Some(viewport_output) = output.viewport_output.remove(&egui::ViewportId::ROOT) {
            let input = self.state.egui_input_mut();
            let viewport_info = input.viewports.entry(egui::ViewportId::ROOT).or_default();

            egui_winit::process_viewport_commands(
                &self.egui_ctx,
                viewport_info,
                viewport_output.commands,
                self.window.window(),
                &mut Default::default(),
            );
        }

        output
    }
}

impl fmt::Debug for UserInterfaceInputManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserInterfaceInputManager").finish()
    }
}
