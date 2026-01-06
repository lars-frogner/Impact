//! Running the basic application.

pub mod ffi;

use crate::{ENGINE, UserInterface, VoxelGeneratorApp, VoxelGeneratorConfig, editor::Editor};
use anyhow::{Result, bail};
use impact::{
    command::UserCommand,
    engine::Engine,
    impact_ecs::{component::ComponentID, world::EntityID},
    roc_integration::Roc,
    run::window,
};
use impact_dev_ui::{UICommand, UICommandQueue, UserInterface as DevUserInterface};
use std::{path::Path, sync::Arc};

pub static UI_COMMANDS: UICommandQueue = UICommandQueue::new();

pub fn run_with_config_at_path(config_path: impl AsRef<Path>) -> Result<()> {
    run_with_config(VoxelGeneratorConfig::from_ron_file(config_path)?)
}

pub fn run_with_config(config: VoxelGeneratorConfig) -> Result<()> {
    env_logger::init();
    log::debug!("Running application");

    let (editor_config, window_config, runtime_config, engine_config, dev_ui_config) =
        config.load()?;

    let editor = Editor::new(editor_config);
    let dev_ui = DevUserInterface::new(dev_ui_config);
    let user_interface = UserInterface::new(editor, dev_ui);

    let app = Arc::new(VoxelGeneratorApp::new(user_interface));

    window::run(app, window_config, runtime_config, engine_config)
}

pub fn execute_engine_command(command_bytes: &[u8]) -> Result<()> {
    log::trace!("Executing engine command");
    let command = UserCommand::from_roc_bytes(command_bytes)?;
    with_engine(|engine| {
        engine.enqueue_user_command(command);
        Ok(())
    })
}

pub fn execute_ui_command(command_bytes: &[u8]) -> Result<()> {
    log::trace!("Executing UI command");
    let command = UICommand::from_roc_bytes(command_bytes)?;
    UI_COMMANDS.enqueue_command(command);
    Ok(())
}

pub fn stage_entity_for_creation_with_id(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entity for creation with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| {
        engine.stage_entity_for_creation_with_id(EntityID::from_u64(entity_id), components)
    })
}

pub fn stage_entity_for_creation(component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entity for creation");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| engine.stage_entity_for_creation(components))
}

pub fn stage_entities_for_creation(component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entities for creation");
    let components = impact::ffi::deserialize_components_for_multiple_entities(component_bytes)?;
    with_engine(|engine| engine.stage_entities_for_creation(components))
}

pub fn stage_entity_for_update(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entity with ID {entity_id} for update");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| {
        engine.stage_entity_for_update(EntityID::from_u64(entity_id), components);
        Ok(())
    })
}

pub fn stage_entity_for_removal(entity_id: u64) -> Result<()> {
    log::trace!("Staging entity with ID {entity_id} for removal");
    with_engine(|engine| {
        engine.stage_entity_for_removal(EntityID::from_u64(entity_id));
        Ok(())
    })
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

pub fn update_entity(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Updating entity with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    with_engine(|engine| engine.update_entity(EntityID::from_u64(entity_id), components))
}

pub fn remove_entity(entity_id: u64) -> Result<()> {
    log::trace!("Removing entity with ID {entity_id}");
    with_engine(|engine| engine.remove_entity(EntityID::from_u64(entity_id)))
}

pub fn for_entity_components(
    entity_id: u64,
    only_component_ids: &[u64],
    f: &mut impl FnMut(&[u8]),
) -> Result<()> {
    log::trace!("Reading components of entity with ID {entity_id}");

    let entity_id = EntityID::from_u64(entity_id);
    let only_component_ids = only_component_ids
        .iter()
        .copied()
        .map(ComponentID::from_u64);

    let mut buffer = Vec::new();

    with_engine(|engine| {
        engine.for_entity_components(entity_id, only_component_ids, &mut |component| {
            buffer.clear();
            impact::ffi::serialize_component_for_entity(component, &mut buffer);
            f(&buffer);
        })
    })
}

fn with_engine<T>(f: impl FnOnce(&Engine) -> Result<T>) -> Result<T> {
    let engine = ENGINE.read();
    match engine.as_ref() {
        Some(engine) => f(engine),
        None => bail!("Tried to use engine before it was initialized"),
    }
}
