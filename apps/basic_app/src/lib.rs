//! A basic Impact application.

pub mod api;
pub mod scripting;
pub mod ui;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::Result;
use impact::{
    application::Application,
    egui,
    engine::{Engine, EngineConfig},
    game_loop::GameLoop,
    runtime::RuntimeConfig,
    window::{
        WindowConfig,
        input::{key::KeyboardEvent, mouse::MouseButtonEvent},
    },
};
use std::sync::RwLock;
use ui::UserInterface;

#[derive(Debug)]
pub struct Game {
    pub engine_config: EngineConfig,
    pub scripts: (),
    pub user_interface: RwLock<UserInterface>,
}

impl Application for Game {
    fn window_config(&self) -> WindowConfig {
        WindowConfig::default()
    }

    fn runtime_config(&self) -> RuntimeConfig {
        RuntimeConfig::default()
    }

    fn engine_config(&self) -> EngineConfig {
        self.engine_config.clone()
    }

    fn run_ui(&self, ctx: &egui::Context, game_loop: &GameLoop, engine: &Engine) {
        self.user_interface
            .write()
            .unwrap()
            .run(ctx, game_loop, engine);
    }

    fn setup_scene(&self) -> Result<()> {
        log::debug!("Setting up scene");
        scripting::setup_scene()
    }

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        log::trace!("Handling keyboard event {event:?}");
        scripting::handle_keyboard_event(event)
    }

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()> {
        log::trace!("Handling mouse button event {event:?}");
        scripting::handle_mouse_button_event(event)
    }
}
