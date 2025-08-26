//! Interfacing between the window system and the engine.

use crate::{
    engine::Engine,
    window::input::{key::KeyboardEvent, mouse::MouseButtonEvent},
};
use anyhow::Result;
use winit::event::{DeviceEvent, WindowEvent};

impl Engine {
    pub(crate) fn handle_window_event(&self, event: &WindowEvent) -> Result<()> {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(event) = KeyboardEvent::from_winit(event.clone()) {
                    self.app().handle_keyboard_event(event)
                } else {
                    Ok(())
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if let Some(event) = MouseButtonEvent::from_winit(*button, *state) {
                    self.app().handle_mouse_button_event(event)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn handle_device_event(&self, event: &DeviceEvent) -> Result<()> {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.handle_mouse_motion_event(*delta);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_mouse_motion_event(&self, mouse_displacement: (f64, f64)) {
        self.update_orientation_controller(mouse_displacement);
    }
}
