//! Interfacing with the application using the engine.

use crate::engine::EngineConfig;
use anyhow::Result;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn engine_config(&self) -> EngineConfig;

    fn setup_scene(&self) -> Result<()>;
}
