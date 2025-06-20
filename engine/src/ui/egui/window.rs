//! Window integration for [`egui`] based user interfaces.

use super::EguiUserInterface;
use crate::{
    ui::window::{ResponsiveUserInterface, UIEventHandlingResponse},
    window::Window,
};
use std::fmt;
use winit::event::{DeviceEvent, WindowEvent};

pub struct EguiWindowIntegration {
    state: egui_winit::State,
}

impl EguiWindowIntegration {
    pub fn new(egui_ctx: egui::Context, window: &Window) -> Self {
        let state = egui_winit::State::new(
            egui_ctx,
            egui::ViewportId::ROOT,
            window.window(),
            Some(window.pixels_per_point() as f32),
            window.window().theme(),
            None,
        );
        Self { state }
    }

    pub fn handle_window_event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
    ) -> UIEventHandlingResponse {
        let egui_winit::EventResponse {
            consumed,
            repaint: _, // We always repaint
        } = self.state.on_window_event(window.window(), event);

        UIEventHandlingResponse {
            event_consumed: consumed,
        }
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.state.on_mouse_motion(*delta);
        }
    }

    pub fn take_raw_input(&mut self, egui_ctx: &egui::Context, window: &Window) -> egui::RawInput {
        let input = self.state.egui_input_mut();
        let viewport_info = input.viewports.entry(egui::ViewportId::ROOT).or_default();

        egui_winit::update_viewport_info(viewport_info, egui_ctx, window.window(), false);

        self.state.take_egui_input(window.window())
    }

    pub fn handle_full_output(
        &mut self,
        egui_ctx: &egui::Context,
        window: &Window,
        mut output: egui::FullOutput,
    ) -> egui::FullOutput {
        self.state
            .handle_platform_output(window.window(), output.platform_output.take());

        if let Some(viewport_output) = output.viewport_output.remove(&egui::ViewportId::ROOT) {
            let input = self.state.egui_input_mut();
            let viewport_info = input.viewports.entry(egui::ViewportId::ROOT).or_default();

            egui_winit::process_viewport_commands(
                egui_ctx,
                viewport_info,
                viewport_output.commands,
                window.window(),
                &mut Default::default(),
            );
        }

        output
    }
}

impl fmt::Debug for EguiWindowIntegration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserInterfaceWindowIntegration").finish()
    }
}

impl ResponsiveUserInterface for EguiUserInterface {
    fn handle_window_event(&self, event: &WindowEvent) -> UIEventHandlingResponse {
        self.window_integration
            .lock()
            .unwrap()
            .handle_window_event(&self.window, event)
    }

    fn handle_device_event(&self, event: &DeviceEvent) {
        self.window_integration
            .lock()
            .unwrap()
            .handle_device_event(event);
    }
}
