//! A headless (no window) Impact application.

pub mod api;
pub mod scripting;

pub use impact;

#[cfg(feature = "roc_codegen")]
pub use impact::{component::gather_roc_type_ids_for_all_components, roc_integration};

use anyhow::Result;
use impact::{
    application::Application,
    engine::EngineConfig,
    runtime::{RuntimeConfig, headless::HeadlessConfig},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct App;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub headless: HeadlessConfig,
    pub runtime: RuntimeConfig,
    pub engine_config_path: PathBuf,
}

impl Application for App {
    fn setup_scene(&self) -> Result<()> {
        log::debug!("Setting up scene");
        scripting::setup_scene()
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

    pub fn load(self) -> Result<(HeadlessConfig, RuntimeConfig, EngineConfig)> {
        let Self {
            headless,
            runtime,
            engine_config_path,
        } = self;

        let engine = EngineConfig::from_ron_file(engine_config_path)?;

        Ok((headless, runtime, engine))
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.engine_config_path = root_path.join(&self.engine_config_path);
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            headless: HeadlessConfig::default(),
            runtime: RuntimeConfig::default(),
            engine_config_path: PathBuf::from("engine_config.roc"),
        }
    }
}
