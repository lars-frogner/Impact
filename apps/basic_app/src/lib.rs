//! A basic Impact application.

pub mod api;
pub mod scripting;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::Result;
use impact::{
    application::Application,
    egui,
    engine::{Engine, EngineConfig},
    runtime::RuntimeConfig,
    window::{
        WindowConfig,
        input::{key::KeyboardEvent, mouse::MouseButtonEvent},
    },
};
use impact_dev_ui::{UserInterface, UserInterfaceConfig};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::RwLock,
};

#[derive(Debug)]
pub struct BasicApp {
    user_interface: RwLock<UserInterface>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BasicAppConfig {
    pub window: WindowConfig,
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
    pub ui_config_path: PathBuf,
}

impl BasicApp {
    pub fn new(user_interface: UserInterface) -> Self {
        Self {
            user_interface: RwLock::new(user_interface),
        }
    }
}

impl Application for BasicApp {
    fn setup_ui(&self, engine: &Engine) {
        self.user_interface.read().unwrap().setup(engine);
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

    fn run_egui_ui(
        &self,
        ctx: &egui::Context,
        input: egui::RawInput,
        engine: &Engine,
    ) -> egui::FullOutput {
        self.user_interface
            .write()
            .unwrap()
            .run(ctx, input, engine, &api::UI_COMMANDS)
    }
}

impl BasicAppConfig {
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
        WindowConfig,
        RuntimeConfig,
        EngineConfig,
        UserInterfaceConfig,
    )> {
        let Self {
            window,
            runtime,
            engine_config_path,
            ui_config_path,
        } = self;

        let engine = EngineConfig::from_ron_file(engine_config_path)?;
        let ui = UserInterfaceConfig::from_ron_file(ui_config_path)?;

        Ok((window, runtime, engine, ui))
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.engine_config_path = root_path.join(&self.engine_config_path);
        self.ui_config_path = root_path.join(&self.ui_config_path);
    }
}

impl Default for BasicAppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            runtime: RuntimeConfig::default(),
            engine_config_path: PathBuf::from("engine_config.roc"),
            ui_config_path: PathBuf::from("ui_config.roc"),
        }
    }
}
