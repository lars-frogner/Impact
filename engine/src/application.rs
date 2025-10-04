//! Interfacing with the application using the engine.

use crate::{
    engine::Engine,
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
    },
};
use anyhow::Result;
use bumpalo::Bump;
use std::sync::Arc;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn on_engine_initialized(&self, arena: &Bump, engine: Arc<Engine>) -> Result<()>;

    fn on_new_frame(&self, _arena: &Bump, _engine: &Engine, _frame_number: u64) -> Result<()> {
        Ok(())
    }

    fn on_shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn handle_keyboard_event(&self, _arena: &Bump, _event: KeyboardEvent) -> Result<()> {
        Ok(())
    }

    fn handle_mouse_button_event(&self, _arena: &Bump, _event: MouseButtonEvent) -> Result<()> {
        Ok(())
    }

    fn handle_mouse_drag_event(&self, _arena: &Bump, _event: MouseDragEvent) -> Result<()> {
        Ok(())
    }

    fn handle_mouse_scroll_event(&self, _arena: &Bump, _event: MouseScrollEvent) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "egui")]
    fn run_egui_ui(
        &self,
        arena: &Bump,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput;
}
