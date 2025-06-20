//! Interfacing with the application using the engine.

use crate::engine::Engine;
use anyhow::Result;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn setup_ui(&self, engine: &Engine);

    fn setup_scene(&self) -> Result<()>;

    #[cfg(feature = "window")]
    fn handle_keyboard_event(&self, event: crate::window::input::key::KeyboardEvent) -> Result<()>;

    #[cfg(feature = "window")]
    fn handle_mouse_button_event(
        &self,
        event: crate::window::input::mouse::MouseButtonEvent,
    ) -> Result<()>;

    #[cfg(feature = "window")]
    fn run_egui_ui(
        &self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput;
}
