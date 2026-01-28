//! A basic Impact application.

pub mod interface;
pub mod user_interface;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::Result;
use impact::{
    engine::{Engine, EngineConfig},
    impact_io,
    runtime::RuntimeConfig,
    window::WindowConfig,
};
use impact_dev_ui::UserInterfaceConfig;
use interface::scripting::hot_reloading::ScriptReloader;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use user_interface::UserInterface;

#[derive(Debug)]
pub struct App {
    app_options: AppOptions,
    user_interface: UserInterface,
    script_reloader: Option<ScriptReloader>,
    engine: Option<Arc<Engine>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub app_options: AppOptions,
    pub run_mode: RunMode,
    pub window: WindowConfig,
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
    pub ui_config_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct AppOptions {
    reset_scene_on_reload: bool,
    #[serde(skip)]
    scene_reset_requested: bool,
    #[serde(skip)]
    show_app_options: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunMode {
    #[default]
    Windowed,
    Headless,
}

impl App {
    pub(crate) fn new(app_options: AppOptions, user_interface: UserInterface) -> Self {
        Self {
            app_options,
            user_interface,
            script_reloader: None,
            engine: None,
        }
    }

    pub(crate) fn engine(&self) -> &Engine {
        self.engine
            .as_ref()
            .expect("Tried to use engine before initialization")
    }
}

impl AppConfig {
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
        AppOptions,
        RunMode,
        WindowConfig,
        RuntimeConfig,
        EngineConfig,
        UserInterfaceConfig,
    )> {
        let Self {
            app_options,
            run_mode,
            window,
            runtime,
            engine_config_path,
            ui_config_path,
        } = self;

        let engine = EngineConfig::from_ron_file(engine_config_path)?;
        let ui = UserInterfaceConfig::from_ron_file(ui_config_path)?;

        Ok((app_options, run_mode, window, runtime, engine, ui))
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.engine_config_path = root_path.join(&self.engine_config_path);
        self.ui_config_path = root_path.join(&self.ui_config_path);
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_options: AppOptions::default(),
            run_mode: RunMode::default(),
            window: WindowConfig::default(),
            runtime: RuntimeConfig::default(),
            engine_config_path: PathBuf::from("engine_config.roc"),
            ui_config_path: PathBuf::from("ui_config.roc"),
        }
    }
}
