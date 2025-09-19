//! Interfacing with the application using the engine.

use crate::{
    engine::Engine,
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent},
    },
};
use anyhow::Result;
use std::sync::Arc;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn on_engine_initialized(&self, engine: Arc<Engine>) -> Result<()>;

    fn on_new_frame(&self, _engine: &Engine, _frame_number: u64) -> Result<()> {
        Ok(())
    }

    fn on_shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn handle_keyboard_event(&self, _event: KeyboardEvent) -> Result<()> {
        Ok(())
    }

    fn handle_mouse_button_event(&self, _event: MouseButtonEvent) -> Result<()> {
        Ok(())
    }

    fn handle_mouse_drag_event(&self, _event: MouseDragEvent) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "egui")]
    fn run_egui_ui(
        &self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput;
}
