//! Interfacing with the application using the engine.

use crate::{
    engine::EngineConfig,
    game_loop::GameLoopConfig,
    window::input::{key::KeyboardEvent, mouse::MouseButtonEvent},
};
use anyhow::Result;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn game_loop_config(&self) -> GameLoopConfig;

    fn engine_config(&self) -> EngineConfig;

    fn setup_scene(&self) -> Result<()>;

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()>;

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()>;
}
