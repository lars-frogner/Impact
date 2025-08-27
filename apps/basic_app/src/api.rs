//! Running the basic application.

pub mod ffi;

use crate::{BasicApp, BasicAppConfig, ENGINE, RunMode};
use anyhow::{Result, bail};
use impact::{
    command::UserCommand,
    engine::Engine,
    impact_ecs::world::EntityID,
    roc_integration::Roc,
    run::{headless, window},
    runtime::headless::HeadlessConfig,
};
use impact_dev_ui::{UICommand, UICommandQueue, UserInterface};
use std::{path::Path, sync::Arc};

pub static UI_COMMANDS: UICommandQueue = UICommandQueue::new();

pub fn run_with_config_at_path(config_path: impl AsRef<Path>) -> Result<()> {
    run_with_config(BasicAppConfig::from_ron_file(config_path)?)
}

pub fn run_with_config(config: BasicAppConfig) -> Result<()> {
    env_logger::init();
    impact_log::debug!("Running application");

    let (run_mode, window_config, runtime_config, engine_config, ui_config) = config.load()?;

    let user_interface = UserInterface::new(ui_config);
    let app = Arc::new(BasicApp::new(user_interface));

    match run_mode {
        RunMode::Windowed => window::run(app, window_config, runtime_config, engine_config),
        RunMode::Headless => {
            let headless_config = HeadlessConfig {
                surface_size: window_config.initial_size,
            };
            headless::run(app, headless_config, runtime_config, engine_config)
        }
    }
}

pub fn execute_engine_command(command_bytes: &[u8]) -> Result<()> {
    impact_log::trace!("Executing engine command");
    let command = UserCommand::from_roc_bytes(command_bytes)?;
    with_engine(|engine| {
        engine.enqueue_user_command(command);
        Ok(())
    })
}

pub fn execute_ui_command(command_bytes: &[u8]) -> Result<()> {
    impact_log::trace!("Executing UI command");
    let command = UICommand::from_roc_bytes(command_bytes)?;
    UI_COMMANDS.enqueue_command(command);
    Ok(())
}

pub fn stage_entity_for_creation_with_id(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    impact_log::trace!("Staging entity for creation with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| {
        engine.stage_entity_for_creation_with_id(EntityID::from_u64(entity_id), components)
    })
}

pub fn stage_entity_for_creation(component_bytes: &[u8]) -> Result<()> {
    impact_log::trace!("Staging entity for creation");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| engine.stage_entity_for_creation(components))
}

pub fn stage_entities_for_creation(component_bytes: &[u8]) -> Result<()> {
    impact_log::trace!("Staging entities for creation");
    let components = impact::ffi::deserialize_components_for_multiple_entities(component_bytes)?;
    with_engine(|engine| engine.stage_entities_for_creation(components))
}

pub fn stage_entity_for_removal(entity_id: u64) -> Result<()> {
    impact_log::trace!("Staging entity for removal");
    with_engine(|engine| {
        engine.stage_entity_for_removal(EntityID::from_u64(entity_id));
        Ok(())
    })
}

pub fn create_entity_with_id(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    impact_log::trace!("Creating entity with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| engine.create_entity_with_id(EntityID::from_u64(entity_id), components))
}

pub fn create_entity(component_bytes: &[u8]) -> Result<u64> {
    impact_log::trace!("Creating entity");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    let entity_id = with_engine(|engine| engine.create_entity(components))?;
    Ok(entity_id.as_u64())
}

pub fn create_entities(component_bytes: &[u8]) -> Result<impl Iterator<Item = u64>> {
    impact_log::trace!("Creating multiple entities");
    let components = impact::ffi::deserialize_components_for_multiple_entities(component_bytes)?;
    let entity_ids = with_engine(|engine| engine.create_entities(components))?;
    Ok(entity_ids.into_iter().map(|entity_id| entity_id.as_u64()))
}

pub fn remove_entity(entity_id: u64) -> Result<()> {
    impact_log::trace!("Removing entity");
    with_engine(|engine| engine.remove_entity(EntityID::from_u64(entity_id)))
}

fn with_engine<T>(f: impl FnOnce(&Engine) -> Result<T>) -> Result<T> {
    let engine = ENGINE.read();
    match engine.as_ref() {
        Some(engine) => f(engine),
        None => bail!("Tried to use engine before it was initialized"),
    }
}
