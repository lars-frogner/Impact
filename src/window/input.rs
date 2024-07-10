//! Input handling.

use crate::{
    application::Application,
    control::motion::{MotionDirection, MotionState},
    gpu::texture::attachment::RenderAttachmentQuantity,
    window::EventLoopController,
};
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use winit::{
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

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
pub struct KeyActionMap(HashMap<KeyCode, KeyboardInputAction>);

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
    ToggleBackFaceCulling,
    ToggleTriangleFill,
    ToggleShadowMapping,
    CycleMSAA,
    ToggleAmbientOcclusion,
    ToggleBloom,
    CycleToneMapping,
    IncreaseExposure,
    DecreaseExposure,
    IncrementSimulationSubstepCount,
    DecrementSimulationSubstepCount,
    IncreaseSimulationSpeed,
    DecreaseSimulationSpeed,
    CycleSimulationSteppingScheme,
    SaveScreenshot,
    SaveDepthMap,
    SaveOmnidirectionalLightShadowMap,
    SaveUnidirectionalLightShadowMap,
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
        [$((KeyCode::$key, KeyboardInputAction::$action),)*].into_iter().collect::<HashMap<_, _>>()
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

    /// Takes a window event and possibly performs an action in the application.
    ///
    /// If no errors occur, returns a [`HandlingResult`] that signals whether
    /// the event should be handled by some other system instead.
    pub fn handle_window_event(
        &self,
        app: &Arc<Application>,
        event_loop_controller: &EventLoopController<'_>,
        event: &WindowEvent,
    ) -> Result<HandlingResult> {
        match event {
            // Handle keyboard input events
            WindowEvent::KeyboardInput { event, .. } => {
                self.key_handler
                    .handle_event(app, event_loop_controller, event)
            }
            _ => Ok(HandlingResult::Unhandled),
        }
    }

    /// Takes a device event and possibly performs an action in the application.
    ///
    /// If no errors occur, returns a [`HandlingResult`] that signals whether
    /// the event should be handled by some other system instead.
    pub fn handle_device_event(
        &self,
        app: &Arc<Application>,
        _event_loop_controller: &EventLoopController<'_>,
        event: &DeviceEvent,
    ) -> Result<HandlingResult> {
        match event {
            // Handle cursor movement events
            DeviceEvent::MouseMotion { delta } => MouseInputHandler::handle_event(app, *delta),
            _ => Ok(HandlingResult::Unhandled),
        }
    }
}

impl MouseInputHandler {
    fn handle_event(app: &Application, mouse_displacement: (f64, f64)) -> Result<HandlingResult> {
        if app.control_mode_active() {
            app.update_orientation_controller(mouse_displacement);
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
        app: &Application,
        event_loop_controller: &EventLoopController<'_>,
        key_input_event: &KeyEvent,
    ) -> Result<HandlingResult> {
        match key_input_event {
            KeyEvent {
                state,
                physical_key: PhysicalKey::Code(key),
                ..
            } => match self.key_map.action_for_key(key) {
                Some(action) => match action {
                    KeyboardInputAction::Exit => {
                        event_loop_controller.exit();
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleInteractionMode => {
                        if state == &ElementState::Released {
                            app.toggle_interaction_mode();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleBackFaceCulling => {
                        if state == &ElementState::Released {
                            app.renderer().write().unwrap().toggle_back_face_culling();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleTriangleFill => {
                        if state == &ElementState::Released {
                            app.renderer().write().unwrap().toggle_triangle_fill();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleShadowMapping => {
                        if state == &ElementState::Released {
                            app.renderer().write().unwrap().toggle_shadow_mapping();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::CycleMSAA => {
                        if state == &ElementState::Released {
                            app.renderer().write().unwrap().cycle_msaa();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleAmbientOcclusion => {
                        if state == &ElementState::Released {
                            app.renderer().read().unwrap().toggle_ambient_occlusion();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleBloom => {
                        if state == &ElementState::Released {
                            app.renderer().read().unwrap().toggle_bloom();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::CycleToneMapping => {
                        if state == &ElementState::Released {
                            app.renderer().read().unwrap().cycle_tone_mapping();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::IncreaseExposure => {
                        if state == &ElementState::Released {
                            app.increase_exposure();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::DecreaseExposure => {
                        if state == &ElementState::Released {
                            app.decrease_exposure();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::IncrementSimulationSubstepCount => {
                        if state == &ElementState::Released {
                            app.simulator().write().unwrap().increment_n_substeps();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::DecrementSimulationSubstepCount => {
                        if state == &ElementState::Released {
                            app.simulator().write().unwrap().decrement_n_substeps();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::IncreaseSimulationSpeed => {
                        if state == &ElementState::Released {
                            app
                                .increment_simulation_speed_multiplier_and_compensate_controller_speed();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::DecreaseSimulationSpeed => {
                        if state == &ElementState::Released {
                            app
                                .decrement_simulation_speed_multiplier_and_compensate_controller_speed();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::CycleSimulationSteppingScheme => {
                        if state == &ElementState::Released {
                            app.cycle_simulation_stepping_scheme();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::SaveScreenshot => {
                        if state == &ElementState::Released {
                            app.screen_capturer().request_screenshot_save();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::SaveDepthMap => {
                        if state == &ElementState::Released {
                            app.screen_capturer()
                                .request_render_attachment_quantity_save(
                                    RenderAttachmentQuantity::Depth,
                                );
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::SaveOmnidirectionalLightShadowMap => {
                        if state == &ElementState::Released {
                            app.screen_capturer()
                                .request_omnidirectional_light_shadow_map_save();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::SaveUnidirectionalLightShadowMap => {
                        if state == &ElementState::Released {
                            app.screen_capturer()
                                .request_unidirectional_light_shadow_map_save();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    // Check if the input is for the motion controller,
                    // and if so, performed the required motion update
                    action if app.control_mode_active() => {
                        match MotionDirection::try_from_input_action(action) {
                            Some(direction) => {
                                app.update_motion_controller(
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
    pub fn new(map: HashMap<KeyCode, KeyboardInputAction>) -> Self {
        Self(map)
    }

    fn action_for_key(&self, key: &KeyCode) -> Option<KeyboardInputAction> {
        self.0.get(key).cloned()
    }
}

impl Default for KeyActionMap {
    fn default() -> Self {
        Self::new(def_key_action_map!(
            // Since camera looks towards -z, we invert the inputs
            // so that pressing W makes us appear to move forwards
            MoveForwards => KeyS,
            MoveBackwards => KeyW,
            MoveRight => KeyD,
            MoveLeft => KeyA,
            MoveUp => KeyQ,
            MoveDown => KeyE,
            ToggleInteractionMode => Tab,
            ToggleBackFaceCulling => KeyB,
            ToggleTriangleFill => KeyF,
            ToggleShadowMapping => KeyI,
            CycleMSAA => KeyY,
            ToggleAmbientOcclusion => KeyO,
            ToggleBloom => KeyU,
            CycleToneMapping => KeyT,
            IncreaseExposure => KeyX,
            DecreaseExposure => KeyZ,
            IncrementSimulationSubstepCount => KeyM,
            DecrementSimulationSubstepCount => KeyN,
            IncreaseSimulationSpeed => Period,
            DecreaseSimulationSpeed => Comma,
            CycleSimulationSteppingScheme => KeyL,
            SaveScreenshot => F12,
            SaveDepthMap => F11,
            SaveOmnidirectionalLightShadowMap => F10,
            SaveUnidirectionalLightShadowMap => F9,
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
