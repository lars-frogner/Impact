//! Running the Impact game.

pub mod ffi;

use crate::Game;
use anyhow::{Result, bail};
use impact::{
    engine::{Engine, EngineConfig, command::EngineCommand},
    roc_integration::Roc,
    run::run as run_engine,
};
use impact_ecs::world::EntityID;
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
    log::trace!("Executing engine command");
    let command = EngineCommand::from_roc_bytes(command_bytes)?;
    with_engine(|engine| engine.execute_command(command))
}

pub fn create_entity_with_id(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Creating entity with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| engine.create_entity_with_id(EntityID::from_u64(entity_id), components))
}

pub fn create_entity(component_bytes: &[u8]) -> Result<u64> {
    log::trace!("Creating entity");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    let entity_id = with_engine(|engine| engine.create_entity(components))?;
    Ok(entity_id.as_u64())
}

pub fn create_entities(component_bytes: &[u8]) -> Result<impl Iterator<Item = u64>> {
    log::trace!("Creating multiple entities");
    let components = impact::ffi::deserialize_components_for_multiple_entities(component_bytes)?;
    let entity_ids = with_engine(|engine| engine.create_entities(components))?;
    Ok(entity_ids.into_iter().map(|entity_id| entity_id.as_u64()))
}

fn with_engine<T>(f: impl FnOnce(&Engine) -> Result<T>) -> Result<T> {
    let engine = ENGINE.read().unwrap();
    match engine.as_ref() {
        Some(engine) => f(engine),
        None => bail!("Tried to use engine before it was initialized"),
    }
}
