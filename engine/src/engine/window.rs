//! Interfacing between the window system and the engine.

use crate::{
    engine::Engine,
    input::{
        InputEvent,
        key::KeyboardEvent,
        mouse::{CursorDirection, MouseButtonEvent, MouseMotionEvent, MouseScrollEvent},
    },
    lock_order::{OrderedMutex, OrderedRwLock},
};
use impact_math::angle::{Angle, Radians};
use std::num::NonZeroU32;
use winit::event::{DeviceEvent, MouseScrollDelta, WindowEvent};

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
            WindowEvent::MouseWheel { delta, .. } => {
                let mut input_manager = self.input_manager.olock();

                let (pixel_delta_x, pixel_delta_y) = match delta {
                    &MouseScrollDelta::LineDelta(delta_x, delta_y) => {
                        let pixels_per_line = input_manager.config.pixels_per_scroll_line;
                        (
                            f64::from(delta_x) * pixels_per_line,
                            f64::from(delta_y) * pixels_per_line,
                        )
                    }
                    MouseScrollDelta::PixelDelta(delta) => (delta.x, delta.y),
                };

                let sensitivity = input_manager.config.scroll_sensitivity;
                let delta_x = pixel_delta_x * sensitivity;
                let delta_y = pixel_delta_y * sensitivity;

                input_manager.queue_event(InputEvent::MouseScroll(MouseScrollEvent {
                    delta_x,
                    delta_y,
                }));
            }
            WindowEvent::CursorMoved { position, .. } => {
                let Some(vertical_field_of_view) = self.current_vertical_field_of_view() else {
                    return;
                };
                let (window_width, window_height) = self.current_window_dimensions();
                let radians_per_pixel = radians_per_pixel(vertical_field_of_view, window_height);

                let ang_x = (position.x - 0.5 * f64::from(window_width.get())) * radians_per_pixel;
                let ang_y =
                    -(position.y - 0.5 * f64::from(window_height.get())) * radians_per_pixel;

                self.input_manager
                    .olock()
                    .queue_event(InputEvent::CursorMoved(CursorDirection { ang_x, ang_y }));
            }
            _ => {}
        }
    }

    pub(crate) fn queue_winit_device_event(&self, event: &DeviceEvent) {
        if let &DeviceEvent::MouseMotion {
            delta: (raw_delta_x, raw_delta_y),
        } = event
        {
            let Some(vertical_field_of_view) = self.current_vertical_field_of_view() else {
                return;
            };
            let (_, window_height) = self.current_window_dimensions();
            let radians_per_pixel = radians_per_pixel(vertical_field_of_view, window_height);

            let mut input_manager = self.input_manager.olock();
            let sensitivity = input_manager.config.mouse_sensitivity;
            let ang_delta_x = raw_delta_x * sensitivity * radians_per_pixel;
            let ang_delta_y = -raw_delta_y * sensitivity * radians_per_pixel;
            input_manager.queue_event(InputEvent::MouseMotion(MouseMotionEvent {
                ang_delta_x,
                ang_delta_y,
            }));
        }
    }

    fn current_vertical_field_of_view(&self) -> Option<Radians> {
        Some(
            self.scene()
                .oread()
                .camera_manager()
                .oread()
                .active_camera()?
                .projection()
                .vertical_field_of_view(),
        )
    }

    fn current_window_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        self.renderer
            .oread()
            .rendering_surface()
            .surface_dimensions()
    }
}

fn radians_per_pixel(vertical_field_of_view: Radians, window_height: NonZeroU32) -> f64 {
    f64::from(vertical_field_of_view.radians()) / f64::from(window_height.get())
}
