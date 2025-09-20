//! Input handling.

pub mod key;
pub mod mouse;

use key::KeyboardEvent;
use mouse::{
    MouseButtonEvent, MouseButtonSet, MouseButtonState, MouseMotionEvent, MouseScrollEvent,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct InputManager {
    pub event_queue: Vec<InputEvent>,
    pub state: InputState,
    pub config: InputConfig,
}

/// Configuration options for input handling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    /// The factor by which to scale the raw mouse motion deltas to obtain pixel
    /// deltas.
    pub mouse_sensitivity: f64,
    /// The factor by which to scale line deltas from the mouse wheel to obtain
    /// pixel deltas.
    pub pixels_per_scroll_line: f64,
    /// The factor by which to scale pixel deltas from the mouse wheel to obtain
    /// the deltas used by the engine.
    pub scroll_sensitivity: f64,
}

#[derive(Clone, Debug)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    MouseButton(MouseButtonEvent),
    MouseMotion(MouseMotionEvent),
    MouseScroll(MouseScrollEvent),
}

#[derive(Clone, Debug)]
pub struct InputState {
    pub pressed_mouse_buttons: MouseButtonSet,
}

impl InputManager {
    pub fn new(config: InputConfig) -> Self {
        Self {
            event_queue: Vec::new(),
            state: InputState::new(),
            config,
        }
    }

    pub fn queue_event(&mut self, event: InputEvent) {
        self.event_queue.push(event);
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            pixels_per_scroll_line: 20.0,
            scroll_sensitivity: 1.0,
        }
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_mouse_buttons: MouseButtonSet::empty(),
        }
    }

    pub fn record_mouse_button_event(
        &mut self,
        MouseButtonEvent { button, state }: MouseButtonEvent,
    ) {
        match state {
            MouseButtonState::Pressed => {
                self.pressed_mouse_buttons.insert(button.into());
            }
            MouseButtonState::Released => {
                self.pressed_mouse_buttons.remove(button.into());
            }
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
