//! Mouse input.

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

/// A press or release of a mouse button.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButtonEvent {
    pub button: MouseButton,
    pub state: MouseButtonState,
}

/// A delta movement of the mouse, expressed in radians across the field of
/// view. Positive `y`-delta is towards the top of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseMotionEvent {
    pub ang_delta_x: f64,
    pub ang_delta_y: f64,
}

/// A delta movement of the mouse, expressed in radians across the field of
/// view. Positive `y`-delta is towards the top of the window. The current
/// camera-space direction of the cursor as well as the set of mouse buttons
/// currently pressed are included for context.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseDragEvent {
    pub ang_delta_x: f64,
    pub ang_delta_y: f64,
    pub cursor: CursorDirection,
    pub pressed: MouseButtonSet,
}

/// A delta movement of the mouse wheel, expressed in pixels scaled by the
/// global scroll sensitivity factor.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseScrollEvent {
    pub delta_x: f64,
    pub delta_y: f64,
}

/// The direction the cursor is pointing in camera space relative to the camera
/// looking direction, expressed in radians along the horizontal and vertical
/// axes of the window. The values are bounded by the horizontal and vertical
/// field of view of the camera.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorDirection {
    pub ang_x: f64,
    pub ang_y: f64,
}

/// A button on a mouse.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

/// Whether a mouse button is pressed or released.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButtonState {
    Pressed,
    Released,
}

bitflags! {
    /// A set of mouse buttons.
    #[roc(parents = "Input", category="bitflags", flags=[LEFT=0, RIGHT=1, MIDDLE=2])]
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
    pub struct MouseButtonSet: u8 {
        const LEFT = 1 << 0;
        const RIGHT = 1 << 1;
        const MIDDLE = 1 << 2;
    }
}

impl MouseButtonEvent {
    /// Returns a `MouseButtonEvent` corresponding to the given `winit`
    /// `MouseButton` and `ElementState`, or [`None`] if the inputs have
    /// no analogous `MouseButtonEvent`.
    #[cfg(feature = "window")]
    pub fn from_winit(
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
    ) -> Option<Self> {
        Some(Self {
            button: MouseButton::from_winit(button)?,
            state: state.into(),
        })
    }
}

impl MouseButton {
    /// Returns the `MouseButton` corresponding to the given `winit`
    /// `MouseButton`, or [`None`] if its variant is not supported.
    #[cfg(feature = "window")]
    pub fn from_winit(button: winit::event::MouseButton) -> Option<Self> {
        Some(match button {
            winit::event::MouseButton::Left => Self::Left,
            winit::event::MouseButton::Right => Self::Right,
            winit::event::MouseButton::Middle => Self::Middle,
            _ => return None,
        })
    }
}

#[cfg(feature = "window")]
impl From<winit::event::ElementState> for MouseButtonState {
    fn from(state: winit::event::ElementState) -> Self {
        match state {
            winit::event::ElementState::Pressed => Self::Pressed,
            winit::event::ElementState::Released => Self::Released,
        }
    }
}

impl From<MouseButton> for MouseButtonSet {
    fn from(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => MouseButtonSet::LEFT,
            MouseButton::Right => MouseButtonSet::RIGHT,
            MouseButton::Middle => MouseButtonSet::MIDDLE,
        }
    }
}
