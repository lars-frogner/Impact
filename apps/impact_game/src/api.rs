//! Running the Impact game.

pub mod ffi;

use crate::Game;
use anyhow::{Result, bail};
use impact::{
    engine::{Engine, EngineConfig, command::EngineCommand},
    roc_codegen::Roc,
    run::run as run_engine,
};
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

static ENGINE: RwLock<Option<Arc<Engine>>> = RwLock::new(None);

pub fn run_with_config_at_path(config_path: impl AsRef<Path>) -> Result<()> {
    run_with_config(EngineConfig::from_ron_file(config_path)?)
}

pub fn run_with_config(config: EngineConfig) -> Result<()> {
    env_logger::init();

    log::debug!("Running game");
    let game = Arc::new(Game {
        engine_config: config,
        scripts: (),
    });
    run_engine(game, |engine| {
        *ENGINE.write().unwrap() = Some(engine);
        log::debug!("Engine initialized");
    })
}

pub fn execute_engine_command(command_bytes: &[u8]) -> Result<()> {
    log::debug!("Executing engine command");
    let command = EngineCommand::from_roc_bytes(command_bytes)?;
    with_engine(|engine| engine.execute_command(command))
}

pub fn create_entity(component_bytes: &[u8]) -> Result<u64> {
    log::debug!("Creating entity");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    let entity = with_engine(|engine| engine.create_entity(components))?;
    Ok(entity.as_u64())
}

pub fn create_entities(component_bytes: &[u8]) -> Result<impl Iterator<Item = u64>> {
    log::debug!("Creating multiple entities");
    let components = impact::ffi::deserialize_components_for_multiple_entities(component_bytes)?;
    let entities = with_engine(|engine| engine.create_entities(components))?;
    Ok(entities.into_iter().map(|entity| entity.as_u64()))
}

fn with_engine<T>(f: impl FnOnce(&Engine) -> Result<T>) -> Result<T> {
    let engine = ENGINE.read().unwrap();
    match engine.as_ref() {
        Some(engine) => f(engine),
        None => bail!("Tried to use engine before it was initialized"),
    }
}
