//! Mouse input.

use roc_integration::roc;
use winit::event;

/// A press or release of a mouse button.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButtonEvent {
    button: MouseButton,
    state: MouseButtonState,
}

/// A button on a mouse.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Whether a mouse button is pressed or released.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButtonState {
    Pressed,
    Released,
}

impl MouseButtonEvent {
    /// Returns a `MouseButtonEvent` corresponding to the given `winit`
    /// `MouseButton` and `ElementState`, or [`None`] if the inputs have
    /// no analogous `MouseButtonEvent`.
    pub fn from_winit(button: event::MouseButton, state: event::ElementState) -> Option<Self> {
        Some(Self {
            button: MouseButton::from_winit(button)?,
            state: state.into(),
        })
    }
}

impl MouseButton {
    /// Returns the `MouseButton` corresponding to the given `winit`
    /// `MouseButton`, or [`None`] if its variant is not supported.
    pub fn from_winit(button: event::MouseButton) -> Option<Self> {
        Some(match button {
            event::MouseButton::Left => Self::Left,
            event::MouseButton::Right => Self::Right,
            event::MouseButton::Middle => Self::Middle,
            _ => return None,
        })
    }
}

impl From<event::ElementState> for MouseButtonState {
    fn from(state: event::ElementState) -> Self {
        match state {
            event::ElementState::Pressed => Self::Pressed,
            event::ElementState::Released => Self::Released,
        }
    }
}
