//! The game API accessible from a script or external binary.

pub mod ffi;

use crate::{
    Game, GameConfig, RunMode,
    command::{GAME_COMMANDS, GameCommand},
    interface::{GAME, access_game, engine::GameInterfaceForEngine},
    lookup::GameLookupTarget,
    user_interface::{UI_COMMANDS, UserInterface},
};
use anyhow::Result;
use impact::{
    command::UserCommand,
    impact_ecs::{component::ComponentID, world::EntityID},
    roc_integration::Roc,
    run::{headless, window},
    runtime::headless::HeadlessConfig,
};
use impact_dev_ui::{UICommand, UserInterface as DevUserInterface};
use std::{path::Path, sync::Arc};

pub fn run_with_config_at_path(config_path: impl AsRef<Path>) -> Result<()> {
    run_with_config(GameConfig::from_ron_file(config_path)?)
}

pub fn run_with_config(config: GameConfig) -> Result<()> {
    env_logger::init();
    log::debug!("Running game");

    let (game_options, run_mode, window_config, runtime_config, engine_config, ui_config) =
        config.load()?;

    let user_interface = UserInterface::new(DevUserInterface::new(ui_config));
    let game = Game::new(game_options, user_interface);

    *GAME.write() = Some(game);

    let game_interface = Arc::new(GameInterfaceForEngine);

    match run_mode {
        RunMode::Windowed => {
            window::run(game_interface, window_config, runtime_config, engine_config)
        }
        RunMode::Headless => {
            let headless_config = HeadlessConfig {
                surface_size: window_config.initial_size,
            };
            headless::run(
                game_interface,
                headless_config,
                runtime_config,
                engine_config,
            )
        }
    }
}

pub fn execute_game_command(command_bytes: &[u8]) -> Result<()> {
    log::trace!("Executing game command");
    let command = GameCommand::from_roc_bytes(command_bytes)?;
    GAME_COMMANDS.enqueue_command(command);
    Ok(())
}

pub fn execute_ui_command(command_bytes: &[u8]) -> Result<()> {
    log::trace!("Executing UI command");
    let command = UICommand::from_roc_bytes(command_bytes)?;
    UI_COMMANDS.enqueue_command(command);
    Ok(())
}

pub fn execute_engine_command(command_bytes: &[u8]) -> Result<()> {
    log::trace!("Executing engine command");
    let command = UserCommand::from_roc_bytes(command_bytes)?;
    access_game().engine().enqueue_user_command(command);
    Ok(())
}

pub fn stage_entity_for_creation_with_id(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entity for creation with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    access_game()
        .engine()
        .stage_entity_for_creation_with_id(EntityID::from_u64(entity_id), components)
}

pub fn stage_entity_for_creation(component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entity for creation");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    access_game().engine().stage_entity_for_creation(components)
}

pub fn stage_entities_for_creation(component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entities for creation");
    let components = impact::ffi::deserialize_components_for_multiple_entities(component_bytes)?;
    access_game()
        .engine()
        .stage_entities_for_creation(components)
}

pub fn stage_entity_for_update(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Staging entity with ID {entity_id} for update");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    access_game()
        .engine()
        .stage_entity_for_update(EntityID::from_u64(entity_id), components);
    Ok(())
}

pub fn stage_entity_for_removal(entity_id: u64) -> Result<()> {
    log::trace!("Staging entity with ID {entity_id} for removal");
    access_game()
        .engine()
        .stage_entity_for_removal(EntityID::from_u64(entity_id));
    Ok(())
}

pub fn create_entity_with_id(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Creating entity with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    access_game()
        .engine()
        .create_entity_with_id(EntityID::from_u64(entity_id), components)
}

pub fn create_entity(component_bytes: &[u8]) -> Result<u64> {
    log::trace!("Creating entity");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    let entity_id = access_game().engine().create_entity(components)?;
    Ok(entity_id.as_u64())
}

pub fn create_entities(component_bytes: &[u8]) -> Result<impl Iterator<Item = u64>> {
    log::trace!("Creating multiple entities");
    let components = impact::ffi::deserialize_components_for_multiple_entities(component_bytes)?;
    let entity_ids = access_game().engine().create_entities(components)?;
    Ok(entity_ids.into_iter().map(|entity_id| entity_id.as_u64()))
}

pub fn update_entity(entity_id: u64, component_bytes: &[u8]) -> Result<()> {
    log::trace!("Updating entity with ID {entity_id}");
    let components = impact::ffi::deserialize_components_for_single_entity(component_bytes)?;
    access_game()
        .engine()
        .update_entity(EntityID::from_u64(entity_id), components)
}

pub fn remove_entity(entity_id: u64) -> Result<()> {
    log::trace!("Removing entity with ID {entity_id}");
    access_game()
        .engine()
        .remove_entity(EntityID::from_u64(entity_id))
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

    access_game()
        .engine()
        .for_entity_components(entity_id, only_component_ids, &mut |component| {
            buffer.clear();
            impact::ffi::serialize_component_for_entity(component, &mut buffer);
            f(&buffer);
        })
}
