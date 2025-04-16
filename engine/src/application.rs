//! Interfacing with the application using the engine.

use crate::{engine::EngineConfig, game_loop::GameLoopConfig};
use anyhow::Result;

pub trait Application: Send + Sync + std::fmt::Debug {
    fn game_loop_config(&self) -> GameLoopConfig;

    fn engine_config(&self) -> EngineConfig;

    fn setup_scene(&self) -> Result<()>;
}
