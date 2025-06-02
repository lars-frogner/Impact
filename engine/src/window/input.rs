//! Input handling.

pub mod key;
pub mod mouse;

use crate::{engine::Engine, runtime::EventLoopController};
use anyhow::Result;
use key::KeyboardEvent;
use mouse::MouseButtonEvent;
use winit::event::{DeviceEvent, WindowEvent};

impl Engine {
    pub fn handle_window_event(
        &self,
        _event_loop_controller: &EventLoopController<'_>,
        event: &WindowEvent,
    ) -> Result<()> {
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

    pub fn handle_device_event(
        &self,
        _event_loop_controller: &EventLoopController<'_>,
        event: &DeviceEvent,
    ) -> Result<()> {
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
