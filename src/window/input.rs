//! Input handling.

use std::collections::HashMap;

use crate::{
    control::{MotionDirection, MotionState},
    world::World,
};
use winit::{
    event::{ElementState, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};

/// Handler for any user input events.
#[derive(Clone, Debug, Default)]
pub struct InputHandler {
    key_handler: KeyInputHandler,
}

/// Whether or not an event has been handled by
/// the input handler.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HandlingResult {
    Handled,
    Unhandled,
}

/// A map associating specific keyboard key inputs
/// with the actions they should perform.
#[derive(Clone, Debug)]
pub struct KeyActionMap(HashMap<VirtualKeyCode, KeyboardInputAction>);

/// Actions that can be performed with a keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum KeyboardInputAction {
    MoveForwards,
    MoveBackwards,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    Exit,
}

/// Handler for keyboard input events.
#[derive(Clone, Debug, Default)]
struct KeyInputHandler {
    key_map: KeyActionMap,
}

/// Macro for easing creation of keyboard action maps.
macro_rules! def_key_action_map {
    ($($action:ident => $key:ident),*) => {
        [$((VirtualKeyCode::$key, KeyboardInputAction::$action),)*].into_iter().collect::<HashMap<_, _>>()
    };
}

impl InputHandler {
    /// Creates a new input handler that will use the given
    /// keyboard action map.
    pub fn new(key_map: KeyActionMap) -> Self {
        Self {
            key_handler: KeyInputHandler::new(key_map),
        }
    }

    /// Takes a window event and possibly performs an action
    /// on the world.
    ///
    /// The returned `HandlingResult` signals whether the event
    /// should be handled by some other system instead.
    pub fn handle_event(
        &self,
        world: &mut World,
        control_flow: &mut ControlFlow,
        event: &WindowEvent,
    ) -> HandlingResult {
        match event {
            // Handle keyboard input events
            WindowEvent::KeyboardInput { input, .. } => {
                self.key_handler.handle_event(world, control_flow, input)
            }
            _ => HandlingResult::Unhandled,
        }
    }
}

impl KeyInputHandler {
    fn new(key_map: KeyActionMap) -> Self {
        Self { key_map }
    }

    fn handle_event(
        &self,
        world: &mut World,
        control_flow: &mut ControlFlow,
        key_input_event: &KeyboardInput,
    ) -> HandlingResult {
        match key_input_event {
            KeyboardInput {
                state,
                virtual_keycode: Some(key),
                ..
            } => match self.key_map.action_for_key(*key) {
                Some(action) => match action {
                    KeyboardInputAction::Exit => {
                        *control_flow = ControlFlow::Exit;
                        HandlingResult::Handled
                    }
                    // Check if the input is for the motion controller,
                    // and if so, performed the required motion update
                    action => match MotionDirection::try_from_input_action(action) {
                        Some(direction) => {
                            world.update_motion_controller(
                                MotionState::from_key_state(*state),
                                direction,
                            );
                            HandlingResult::Handled
                        }
                        None => HandlingResult::Unhandled,
                    },
                },
                None => HandlingResult::Unhandled,
            },
            _ => HandlingResult::Unhandled,
        }
    }
}

impl KeyActionMap {
    pub fn new(map: HashMap<VirtualKeyCode, KeyboardInputAction>) -> Self {
        Self(map)
    }

    fn action_for_key(&self, key: VirtualKeyCode) -> Option<KeyboardInputAction> {
        self.0.get(&key).cloned()
    }
}

impl Default for KeyActionMap {
    fn default() -> Self {
        Self::new(def_key_action_map!(
            MoveForwards => W,
            MoveBackwards => S,
            MoveRight => D,
            MoveLeft => A,
            MoveUp => Q,
            MoveDown => E,
            Exit => Escape
        ))
    }
}

impl MotionState {
    fn from_key_state(state: ElementState) -> Self {
        match state {
            ElementState::Pressed => Self::Moving,
            ElementState::Released => Self::Still,
        }
    }
}

impl MotionDirection {
    fn try_from_input_action(action: KeyboardInputAction) -> Option<Self> {
        match action {
            KeyboardInputAction::MoveForwards => Some(Self::Forwards),
            KeyboardInputAction::MoveBackwards => Some(Self::Backwards),
            KeyboardInputAction::MoveRight => Some(Self::Right),
            KeyboardInputAction::MoveLeft => Some(Self::Left),
            KeyboardInputAction::MoveUp => Some(Self::Up),
            KeyboardInputAction::MoveDown => Some(Self::Down),
            _ => None,
        }
    }
}
