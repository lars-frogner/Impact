//! The Impact game.

pub mod api;
pub mod scripting;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::{Context, Result};
use dynamic_lib::DynamicLibrary;
use impact::{
    application::Application,
    egui,
    engine::{Engine, EngineConfig},
    impact_io,
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
    },
    runtime::RuntimeConfig,
    window::WindowConfig,
};
use impact_dev_ui::{UserInterface, UserInterfaceConfig};
use parking_lot::RwLock;
use scripting::ScriptLib;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

static ENGINE: RwLock<Option<Arc<Engine>>> = RwLock::new(None);

#[derive(Debug)]
pub struct Game {
    user_interface: RwLock<UserInterface>,
    #[cfg(feature = "hot_reloading")]
    script_reloader: parking_lot::Mutex<Option<scripting::hot_reloading::ScriptReloader>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameConfig {
    pub run_mode: RunMode,
    pub window: WindowConfig,
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
    pub ui_config_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunMode {
    #[default]
    Windowed,
    Headless,
}

impl Game {
    pub fn new(user_interface: UserInterface) -> Self {
        Self {
            user_interface: RwLock::new(user_interface),
            #[cfg(feature = "hot_reloading")]
            script_reloader: Default::default(),
        }
    }

    #[cfg(feature = "hot_reloading")]
    fn activate_script_reloader(&self) -> Result<()> {
        log::debug!("Activating script reloader");

        let script_reloader = scripting::hot_reloading::create_script_reloader()?;
        *self.script_reloader.lock() = Some(script_reloader);

        Ok(())
    }

    #[cfg(not(feature = "hot_reloading"))]
    #[allow(clippy::unused_self)]
    fn activate_script_reloader(&self) -> Result<()> {
        Ok(())
    }
}

impl Application for Game {
    fn on_engine_initialized(&self, engine: Arc<Engine>) -> Result<()> {
        log::debug!("Loading script library");
        ScriptLib::load().context("Failed to load script library")?;

        self.activate_script_reloader()?;

        *ENGINE.write() = Some(engine.clone());
        log::debug!("Engine initialized");

        log::debug!("Setting up UI");
        self.user_interface.read().setup(&engine);

        log::debug!("Setting up scene");
        scripting::setup_scene()?;

        Ok(())
    }

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        log::trace!("Handling keyboard event {event:?}");
        scripting::handle_keyboard_event(event)
    }

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()> {
        log::trace!("Handling mouse button event {event:?}");
        scripting::handle_mouse_button_event(event)
    }

    fn handle_mouse_drag_event(&self, event: MouseDragEvent) -> Result<()> {
        log::trace!("Handling mouse drag event {event:?}");
        scripting::handle_mouse_drag_event(event)
    }

    fn handle_mouse_scroll_event(&self, event: MouseScrollEvent) -> Result<()> {
        log::trace!("Handling mouse scroll event {event:?}");
        scripting::handle_mouse_scroll_event(event)
    }

    fn run_egui_ui(
        &self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput {
        self.user_interface
            .write()
            .run(ctx, input, engine, &api::UI_COMMANDS)
    }
}

impl GameConfig {
    /// Parses the configuration from the RON file at the given path and
    /// resolves any specified paths.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut config: Self = impact_io::parse_ron_file(file_path)?;
        if let Some(root_path) = file_path.parent() {
            config.resolve_paths(root_path);
        }
        Ok(config)
    }

    pub fn load(
        self,
    ) -> Result<(
        RunMode,
        WindowConfig,
        RuntimeConfig,
        EngineConfig,
        UserInterfaceConfig,
    )> {
        let Self {
            run_mode,
            window,
            runtime,
            engine_config_path,
            ui_config_path,
        } = self;

        let engine = EngineConfig::from_ron_file(engine_config_path)?;
        let ui = UserInterfaceConfig::from_ron_file(ui_config_path)?;

        Ok((run_mode, window, runtime, engine, ui))
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.engine_config_path = root_path.join(&self.engine_config_path);
        self.ui_config_path = root_path.join(&self.ui_config_path);
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            run_mode: RunMode::default(),
            window: WindowConfig::default(),
            runtime: RuntimeConfig::default(),
            engine_config_path: PathBuf::from("engine_config.roc"),
            ui_config_path: PathBuf::from("ui_config.roc"),
        }
    }
}
