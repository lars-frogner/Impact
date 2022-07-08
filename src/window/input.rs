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

#[derive(Clone, Debug, Default)]
pub struct InputHandler {
    key_handler: KeyInputHandler,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HandlingResult {
    Handled,
    Unhandled,
}

#[derive(Clone, Debug)]
pub struct KeyActionMap(HashMap<VirtualKeyCode, BinaryInputAction>);

#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum BinaryInputAction {
    MoveForwards,
    MoveBackwards,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    Exit,
}

#[derive(Clone, Debug, Default)]
struct KeyInputHandler {
    key_map: KeyActionMap,
}

macro_rules! def_key_action_map {
    ($($action:ident => $key:ident),*) => {
        [$((VirtualKeyCode::$key, BinaryInputAction::$action),)*].into_iter().collect::<HashMap<_, _>>()
    };
}

impl InputHandler {
    pub fn new(key_map: KeyActionMap) -> Self {
        Self {
            key_handler: KeyInputHandler::new(key_map),
        }
    }

    pub fn handle_event(
        &self,
        world: &mut World,
        control_flow: &mut ControlFlow,
        event: &WindowEvent,
    ) -> HandlingResult {
        match event {
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
                    BinaryInputAction::Exit => {
                        *control_flow = ControlFlow::Exit;
                        HandlingResult::Handled
                    }
                    action => match MotionDirection::try_from_input_action(action) {
                        Some(direction) => {
                            world.update_motion_controller(
                                direction,
                                MotionState::from_key_state(*state),
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
    pub fn new(map: HashMap<VirtualKeyCode, BinaryInputAction>) -> Self {
        Self(map)
    }

    fn action_for_key(&self, key: VirtualKeyCode) -> Option<BinaryInputAction> {
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
    fn try_from_input_action(action: BinaryInputAction) -> Option<Self> {
        match action {
            BinaryInputAction::MoveForwards => Some(Self::Forwards),
            BinaryInputAction::MoveBackwards => Some(Self::Backwards),
            BinaryInputAction::MoveRight => Some(Self::Right),
            BinaryInputAction::MoveLeft => Some(Self::Left),
            BinaryInputAction::MoveUp => Some(Self::Up),
            BinaryInputAction::MoveDown => Some(Self::Down),
            _ => None,
        }
    }
}
