//! API for running the snapshot tester.

pub mod ffi;

use crate::{AppConfig, ENGINE, SnapshotTester};
use anyhow::{Result, bail};
use impact::{
    engine::{Engine, EngineConfig, command::EngineCommand},
    impact_ecs::world::EntityID,
    roc_integration::Roc,
    run::headless::run as run_engine,
};
use std::{path::Path, sync::Arc};

pub fn run_with_config_at_path(config_path: impl AsRef<Path>) -> Result<()> {
    run_with_config(AppConfig::from_ron_file(config_path)?)
}

pub fn run_with_config(config: AppConfig) -> Result<()> {
    env_logger::init();

    let engine_config = EngineConfig::from_ron_file(config.engine_config_path)?;

    let tester = SnapshotTester::new(config.testing)?;

    run_engine(
        Arc::new(tester),
        config.headless,
        config.runtime,
        engine_config,
    )
}

pub fn execute_engine_command(command_bytes: &[u8]) -> Result<()> {
    let command = EngineCommand::from_roc_bytes(command_bytes)?;
    with_engine(|engine| engine.execute_command(command))
}

pub fn create_entity_with_id(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| engine.create_entity_with_id(EntityID::from_u64(entity_id), components))
}

pub fn create_entity(component_bytes: &[u8]) -> Result<u64> {
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    let entity_id = with_engine(|engine| engine.create_entity(components))?;
    Ok(entity_id.as_u64())
}

pub fn create_entities(component_bytes: &[u8]) -> Result<impl Iterator<Item = u64>> {
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
