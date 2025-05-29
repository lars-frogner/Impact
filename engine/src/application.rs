//! Interfacing with the application using the engine.

use crate::{
    engine::{Engine, EngineConfig},
    game_loop::GameLoop,
    runtime::RuntimeConfig,
    window::{
        WindowConfig,
        input::{key::KeyboardEvent, mouse::MouseButtonEvent},
    },
};
use anyhow::Result;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn window_config(&self) -> WindowConfig;

    fn runtime_config(&self) -> RuntimeConfig;

    fn engine_config(&self) -> EngineConfig;

    fn run_ui(&self, ctx: &egui::Context, game_loop: &GameLoop, engine: &Engine);

    fn setup_scene(&self) -> Result<()>;

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()>;

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()>;
}
