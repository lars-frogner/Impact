//! Running the Impact game.

pub mod ffi;

use crate::Game;
use anyhow::{Result, bail};
use impact::{
    engine::{Engine, EngineConfig},
    gpu::texture::TextureID,
    impact_math::hash32,
    run::run as run_engine,
    skybox::Skybox,
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

pub fn set_skybox(cubemap_texture_name: &str, max_luminance: f32) -> Result<()> {
    log::debug!("Setting skybox to {cubemap_texture_name}");
    with_engine(|engine| {
        engine.set_skybox_for_current_scene(Skybox::new(
            TextureID(hash32!(cubemap_texture_name)),
            max_luminance,
        ));
        Ok(())
    })
}

fn with_engine<T>(f: impl FnOnce(&Engine) -> Result<T>) -> Result<T> {
    let engine = ENGINE.read().unwrap();
    match engine.as_ref() {
        Some(engine) => f(engine),
        None => bail!("Tried to use engine before it was initialized"),
    }
}
