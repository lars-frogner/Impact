//! Input handling.

use crate::{engine::Engine, io::util::parse_ron_file, window::EventLoopController};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use winit::{
    event::{DeviceEvent, ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode,
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
    _key_handler: KeyInputHandler,
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
    _key_map: KeyActionMap,
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
            _key_handler: KeyInputHandler::new(key_map),
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
        _event_loop_controller: &EventLoopController<'_>,
        event: &WindowEvent,
    ) -> Result<HandlingResult> {
        match event {
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
        Self { _key_map: key_map }
    }
}

impl KeyActionMap {
    pub fn new(map: HashMap<KeyCode, KeyboardInputAction>) -> Self {
        Self(map)
    }

    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        parse_ron_file(file_path)
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

impl InputConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(key_map_path) = self.key_map_path.as_mut() {
            *key_map_path = root_path.join(&key_map_path);
        }
    }
}
