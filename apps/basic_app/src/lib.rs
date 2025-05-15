//! The Impact game.

pub mod api;
pub mod scripting;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::Result;
use impact::{
    application::Application,
    engine::EngineConfig,
    game_loop::GameLoopConfig,
    window::input::{key::KeyboardEvent, mouse::MouseButtonEvent},
};

#[derive(Debug)]
pub struct Game {
    pub engine_config: EngineConfig,
    pub scripts: (),
}

impl Application for Game {
    fn game_loop_config(&self) -> GameLoopConfig {
        GameLoopConfig::default()
    }

    fn engine_config(&self) -> EngineConfig {
        self.engine_config.clone()
    }

    fn setup_scene(&self) -> Result<()> {
        log::debug!("Setting up scene");
        scripting::setup_scene()
    }

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        log::debug!("Handling keyboard event {event:?}");
        scripting::handle_keyboard_event(event)
    }

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()> {
        log::debug!("Handling mouse button event {event:?}");
        scripting::handle_mouse_button_event(event)
    }
}
