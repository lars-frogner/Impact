//! Interfacing with the application using the engine.

use crate::{
    engine::Engine,
    game_loop::GameLoop,
    window::input::{key::KeyboardEvent, mouse::MouseButtonEvent},
};
use anyhow::Result;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn setup_ui(&self, engine: &Engine);

    fn run_ui(
        &self,
        ctx: &egui::Context,
        input: egui::RawInput,
        game_loop: &GameLoop,
        engine: &Engine,
    ) -> egui::FullOutput;

    fn setup_scene(&self) -> Result<()>;

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()>;

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()>;
}
