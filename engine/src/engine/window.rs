//! Interfacing between the window system and the engine.

use crate::{
    engine::Engine,
    input::{
        InputEvent,
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseMotionEvent},
    },
    lock_order::{OrderedMutex, OrderedRwLock},
};
use impact_math::Angle;
use winit::event::{DeviceEvent, WindowEvent};

impl Engine {
    pub(crate) fn queue_winit_window_event(&self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(event) = KeyboardEvent::from_winit(event.clone()) {
                    self.input_manager
                        .olock()
                        .queue_event(InputEvent::Keyboard(event));
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if let Some(event) = MouseButtonEvent::from_winit(*button, *state) {
                    self.input_manager
                        .olock()
                        .queue_event(InputEvent::MouseButton(event));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn queue_winit_device_event(&self, event: &DeviceEvent) {
        if let &DeviceEvent::MouseMotion {
            delta: (raw_delta_x, raw_delta_y),
        } = event
        {
            let Some(radians_per_pixel) = self.get_current_radians_per_pixel() else {
                return;
            };

            let mut input_manager = self.input_manager.olock();
            let sensitivity = input_manager.config.mouse_sensitivity;

            let delta_x = raw_delta_x * sensitivity * radians_per_pixel;
            let delta_y = raw_delta_y * sensitivity * radians_per_pixel;

            input_manager.queue_event(InputEvent::MouseMotion(MouseMotionEvent {
                delta_x,
                delta_y,
            }));
        }
    }

    pub(crate) fn get_current_radians_per_pixel(&self) -> Option<f64> {
        let vertical_field_of_view = self
            .scene()
            .oread()
            .camera_manager()
            .oread()
            .active_camera()?
            .camera()
            .vertical_field_of_view();

        let (_, window_height) = self
            .renderer
            .oread()
            .rendering_surface()
            .surface_dimensions();

        Some(f64::from(vertical_field_of_view.radians()) / f64::from(window_height.get()))
    }
}
