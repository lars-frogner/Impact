//! Input handling.

use crate::{
    control::{MotionDirection, MotionState},
    window::ControlFlow,
    world::World,
};
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use winit::event::{DeviceEvent, ElementState, KeyboardInput, VirtualKeyCode, WindowEvent};

/// Handler for any user input events.
#[derive(Clone, Debug)]
pub struct InputHandler {
    key_handler: KeyInputHandler,
}

/// Whether or not an event has been handled by
/// the input handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlingResult {
    Handled,
    Unhandled,
}

/// Handler for mouse input events.
#[derive(Clone, Debug)]
pub struct MouseInputHandler;

/// A map associating specific keyboard key inputs
/// with the actions they should perform.
#[derive(Clone, Debug)]
pub struct KeyActionMap(HashMap<VirtualKeyCode, KeyboardInputAction>);

/// Actions that can be performed with a keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardInputAction {
    MoveForwards,
    MoveBackwards,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    ToggleInteractionMode,
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
    /// If no errors occur, returns a [`HandlingResult`] that signals
    /// whether the event should be handled by some other system instead.
    pub fn handle_window_event(
        &self,
        world: &Arc<World>,
        control_flow: &mut ControlFlow<'_>,
        event: &WindowEvent<'_>,
    ) -> Result<HandlingResult> {
        match event {
            // Handle keyboard input events
            WindowEvent::KeyboardInput { input, .. } => {
                self.key_handler.handle_event(world, control_flow, input)
            }
            _ => Ok(HandlingResult::Unhandled),
        }
    }

    /// Takes a device event and possibly performs an action
    /// on the world.
    ///
    /// If no errors occur, returns a [`HandlingResult`] that signals
    /// whether the event should be handled by some other system instead.
    pub fn handle_device_event(
        &self,
        world: &Arc<World>,
        _control_flow: &mut ControlFlow<'_>,
        event: &DeviceEvent,
    ) -> Result<HandlingResult> {
        match event {
            // Handle cursor movement events
            DeviceEvent::MouseMotion { delta } => MouseInputHandler::handle_event(world, *delta),
            _ => Ok(HandlingResult::Unhandled),
        }
    }
}

impl MouseInputHandler {
    fn handle_event(world: &World, mouse_displacement: (f64, f64)) -> Result<HandlingResult> {
        if world.control_mode_active() {
            world.update_orientation_controller(mouse_displacement);
        }
        Ok(HandlingResult::Handled)
    }
}

impl KeyInputHandler {
    fn new(key_map: KeyActionMap) -> Self {
        Self { key_map }
    }

    fn handle_event(
        &self,
        world: &World,
        control_flow: &mut ControlFlow<'_>,
        key_input_event: &KeyboardInput,
    ) -> Result<HandlingResult> {
        match key_input_event {
            KeyboardInput {
                state,
                virtual_keycode: Some(key),
                ..
            } => match self.key_map.action_for_key(*key) {
                Some(action) => match action {
                    KeyboardInputAction::Exit => {
                        control_flow.exit();
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleInteractionMode => {
                        if state == &ElementState::Released {
                            world.toggle_interaction_mode();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    // Check if the input is for the motion controller,
                    // and if so, performed the required motion update
                    action if world.control_mode_active() => {
                        match MotionDirection::try_from_input_action(action) {
                            Some(direction) => {
                                world.update_motion_controller(
                                    MotionState::from_key_state(*state),
                                    direction,
                                );
                                Ok(HandlingResult::Handled)
                            }
                            None => Ok(HandlingResult::Unhandled),
                        }
                    }
                    _ => Ok(HandlingResult::Handled),
                },
                None => Ok(HandlingResult::Unhandled),
            },
            _ => Ok(HandlingResult::Unhandled),
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
            // Since camera looks towards -z, we invert the inputs
            // so that pressing W makes us appear to move forwards
            MoveForwards => S,
            MoveBackwards => W,
            MoveRight => D,
            MoveLeft => A,
            MoveUp => Q,
            MoveDown => E,
            ToggleInteractionMode => Tab,
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
