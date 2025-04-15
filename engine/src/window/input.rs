//! Input handling.

use crate::{
    control::motion::{MotionDirection, MotionState},
    engine::Engine,
    io::util::parse_ron_file,
    window::EventLoopController,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InputConfig {
    /// Path to the RON file containing the mappings from keyboard keys to
    /// actions. If [`None`], the default mappings will be used.
    #[serde(default)]
    pub key_map_path: Option<PathBuf>,
}

/// Handler for any user input events.
#[derive(Debug, Default)]
pub struct InputHandler {
    key_handler: KeyInputHandler,
    mouse_button_handler: MouseButtonInputHandler,
}

/// Whether or not an event has been handled by
/// the input handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlingResult {
    Handled,
    Unhandled,
}

/// Handler for mouse motion input events.
#[derive(Clone, Debug)]
pub struct MouseMotionInputHandler;

/// Handler for mouse button input events.
#[derive(Default)]
pub struct MouseButtonInputHandler {
    pub left_pressed: Option<MouseButtonInputHandlerFn>,
    pub left_released: Option<MouseButtonInputHandlerFn>,
    pub right_pressed: Option<MouseButtonInputHandlerFn>,
    pub right_released: Option<MouseButtonInputHandlerFn>,
}

pub type MouseButtonInputHandlerFn = Box<dyn Fn(&Engine) -> Result<()> + Send + Sync>;

/// A map associating specific keyboard key inputs
/// with the actions they should perform.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyActionMap(HashMap<KeyCode, KeyboardInputAction>);

/// Actions that can be performed with a keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyboardInputAction {
    MoveForwards,
    MoveBackwards,
    MoveRight,
    MoveLeft,
    MoveUp,
    MoveDown,
    ToggleInteractionMode,
    ToggleWireframeMode,
    ToggleShadowMapping,
    ToggleAmbientOcclusion,
    ToggleTemporalAntiAliasing,
    ToggleBloom,
    CycleToneMapping,
    IncreaseExposure,
    DecreaseExposure,
    ToggleRenderAttachmentVisualization,
    CycleVisualizedRenderAttachmentQuantityForward,
    CycleVisualizedRenderAttachmentQuantityBackward,
    ToggleRenderPassTimings,
    IncrementSimulationSubstepCount,
    DecrementSimulationSubstepCount,
    IncreaseSimulationSpeed,
    DecreaseSimulationSpeed,
    SaveScreenshot,
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
    /// keyboard action map and mouse button input handler.
    pub fn new(key_map: KeyActionMap, mouse_button_handler: MouseButtonInputHandler) -> Self {
        Self {
            key_handler: KeyInputHandler::new(key_map),
            mouse_button_handler,
        }
    }

    /// Creates a new input handler based on the given configuration
    /// parameters.
    pub fn from_config(config: InputConfig) -> Result<Self> {
        let key_map = match config.key_map_path {
            Some(file_path) => KeyActionMap::from_ron_file(file_path)?,
            None => KeyActionMap::default(),
        };
        Ok(Self::new(key_map, MouseButtonInputHandler::default()))
    }

    /// Returns a mutable reference to the [`MouseButtonInputHandler`].
    pub fn mouse_button_handler_mut(&mut self) -> &mut MouseButtonInputHandler {
        &mut self.mouse_button_handler
    }

    /// Takes a window event and possibly performs an action in the engine.
    ///
    /// If no errors occur, returns a [`HandlingResult`] that signals whether
    /// the event should be handled by some other system instead.
    pub fn handle_window_event(
        &self,
        engine: &Arc<Engine>,
        event_loop_controller: &EventLoopController<'_>,
        event: &WindowEvent,
    ) -> Result<HandlingResult> {
        match event {
            // Handle keyboard input events
            WindowEvent::KeyboardInput { event, .. } => {
                self.key_handler
                    .handle_event(engine, event_loop_controller, event)
            }
            WindowEvent::MouseInput { button, state, .. } => self
                .mouse_button_handler
                .handle_event(engine, button, state),
            _ => Ok(HandlingResult::Unhandled),
        }
    }

    /// Takes a device event and possibly performs an action in the engine.
    ///
    /// If no errors occur, returns a [`HandlingResult`] that signals whether
    /// the event should be handled by some other system instead.
    pub fn handle_device_event(
        &self,
        engine: &Arc<Engine>,
        _event_loop_controller: &EventLoopController<'_>,
        event: &DeviceEvent,
    ) -> Result<HandlingResult> {
        match event {
            // Handle cursor movement events
            DeviceEvent::MouseMotion { delta } => {
                MouseMotionInputHandler::handle_event(engine, *delta)
            }
            _ => Ok(HandlingResult::Unhandled),
        }
    }
}

impl MouseMotionInputHandler {
    fn handle_event(engine: &Engine, mouse_displacement: (f64, f64)) -> Result<HandlingResult> {
        if engine.control_mode_active() {
            engine.update_orientation_controller(mouse_displacement);
        }
        Ok(HandlingResult::Handled)
    }
}

impl MouseButtonInputHandler {
    fn handle_event(
        &self,
        engine: &Engine,
        button: &MouseButton,
        state: &ElementState,
    ) -> Result<HandlingResult> {
        match (button, state) {
            (MouseButton::Left, ElementState::Pressed) => {
                if let Some(handler) = &self.left_pressed {
                    handler(engine)?;
                }
                Ok(HandlingResult::Handled)
            }
            (MouseButton::Left, ElementState::Released) => {
                if let Some(handler) = &self.left_released {
                    handler(engine)?;
                }
                Ok(HandlingResult::Handled)
            }
            (MouseButton::Right, ElementState::Pressed) => {
                if let Some(handler) = &self.right_pressed {
                    handler(engine)?;
                }
                Ok(HandlingResult::Handled)
            }
            (MouseButton::Right, ElementState::Released) => {
                if let Some(handler) = &self.right_released {
                    handler(engine)?;
                }
                Ok(HandlingResult::Handled)
            }
            _ => Ok(HandlingResult::Unhandled),
        }
    }
}

impl std::fmt::Debug for MouseButtonInputHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MouseButtonInputHandler").finish()
    }
}

impl KeyInputHandler {
    fn new(key_map: KeyActionMap) -> Self {
        Self { key_map }
    }

    fn handle_event(
        &self,
        engine: &Engine,
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
                            engine.toggle_interaction_mode();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleWireframeMode => {
                        if state == &ElementState::Released {
                            engine.toggle_wireframe_mode();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleShadowMapping => {
                        if state == &ElementState::Released {
                            engine.renderer().write().unwrap().toggle_shadow_mapping();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleAmbientOcclusion => {
                        if state == &ElementState::Released {
                            engine.renderer().read().unwrap().toggle_ambient_occlusion();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleTemporalAntiAliasing => {
                        if state == &ElementState::Released {
                            engine.toggle_temporal_anti_aliasing();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleBloom => {
                        if state == &ElementState::Released {
                            engine.renderer().read().unwrap().toggle_bloom();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::CycleToneMapping => {
                        if state == &ElementState::Released {
                            engine.renderer().read().unwrap().cycle_tone_mapping();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::IncreaseExposure => {
                        if state == &ElementState::Released {
                            engine.increase_camera_sensitivity();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::DecreaseExposure => {
                        if state == &ElementState::Released {
                            engine.decrease_camera_sensitivity();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleRenderAttachmentVisualization => {
                        if state == &ElementState::Released {
                            engine
                                .renderer()
                                .read()
                                .unwrap()
                                .toggle_render_attachment_visualization();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::CycleVisualizedRenderAttachmentQuantityForward => {
                        if state == &ElementState::Released {
                            engine
                                .renderer()
                                .read()
                                .unwrap()
                                .cycle_visualized_render_attachment_quantity_forward();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::CycleVisualizedRenderAttachmentQuantityBackward => {
                        if state == &ElementState::Released {
                            engine
                                .renderer()
                                .read()
                                .unwrap()
                                .cycle_visualized_render_attachment_quantity_backward();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::ToggleRenderPassTimings => {
                        if state == &ElementState::Released {
                            engine.renderer().write().unwrap().toggle_timings();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::IncrementSimulationSubstepCount => {
                        if state == &ElementState::Released {
                            engine.simulator().write().unwrap().increment_n_substeps();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::DecrementSimulationSubstepCount => {
                        if state == &ElementState::Released {
                            engine.simulator().write().unwrap().decrement_n_substeps();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::IncreaseSimulationSpeed => {
                        if state == &ElementState::Released {
                            engine
                                .increment_simulation_speed_multiplier_and_compensate_controller_speed();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::DecreaseSimulationSpeed => {
                        if state == &ElementState::Released {
                            engine
                                .decrement_simulation_speed_multiplier_and_compensate_controller_speed();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::SaveScreenshot => {
                        if state == &ElementState::Released {
                            engine.screen_capturer().request_screenshot_save();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::SaveOmnidirectionalLightShadowMap => {
                        if state == &ElementState::Released {
                            engine
                                .screen_capturer()
                                .request_omnidirectional_light_shadow_map_save();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    KeyboardInputAction::SaveUnidirectionalLightShadowMap => {
                        if state == &ElementState::Released {
                            engine
                                .screen_capturer()
                                .request_unidirectional_light_shadow_map_save();
                        }
                        Ok(HandlingResult::Handled)
                    }
                    // Check if the input is for the motion controller,
                    // and if so, performed the required motion update
                    action if engine.control_mode_active() => {
                        match MotionDirection::try_from_input_action(action) {
                            Some(direction) => {
                                engine.update_motion_controller(
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

    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        parse_ron_file(file_path)
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
            ToggleWireframeMode => KeyF,
            ToggleShadowMapping => KeyI,
            ToggleAmbientOcclusion => KeyO,
            ToggleTemporalAntiAliasing => KeyY,
            ToggleBloom => KeyU,
            CycleToneMapping => KeyT,
            IncreaseExposure => KeyX,
            DecreaseExposure => KeyZ,
            ToggleRenderAttachmentVisualization => KeyV,
            CycleVisualizedRenderAttachmentQuantityForward => KeyB,
            CycleVisualizedRenderAttachmentQuantityBackward => KeyC,
            ToggleRenderPassTimings => KeyP,
            IncrementSimulationSubstepCount => KeyM,
            DecrementSimulationSubstepCount => KeyN,
            IncreaseSimulationSpeed => Period,
            DecreaseSimulationSpeed => Comma,
            SaveScreenshot => F12,
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

impl InputConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(key_map_path) = self.key_map_path.as_mut() {
            *key_map_path = root_path.join(&key_map_path);
        }
    }
}
