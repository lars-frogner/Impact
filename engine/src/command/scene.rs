//! Commands for scene manipulation.

use crate::{command::uils::ActiveState, engine::Engine, lock_order::OrderedRwLock};
use anyhow::Result;
use impact_ecs::world::EntityID;
use impact_physics::medium::UniformMedium;
use impact_scene::skybox::Skybox;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneCommand {
    SetSkybox(Skybox),
    SetMedium(UniformMedium),
    SetSceneEntityActiveState {
        entity_id: EntityID,
        state: ActiveState,
    },
}

pub fn set_skybox(engine: &Engine, skybox: Skybox) {
    impact_log::info!("Setting skybox to {skybox:?}");
    engine.scene().oread().set_skybox(Some(skybox));
}

pub fn set_medium(engine: &Engine, medium: UniformMedium) {
    impact_log::info!("Setting medium to {medium:?}");
    engine.simulator().owrite().set_medium(medium);
}

pub fn set_scene_entity_active_state(
    engine: &Engine,
    entity_id: EntityID,
    state: ActiveState,
) -> Result<()> {
    impact_log::info!("Setting state of scene entity with ID {entity_id} to {state:?}");
    match state {
        ActiveState::Enabled => engine.enable_scene_entity(entity_id),
        ActiveState::Disabled => engine.disable_scene_entity(entity_id),
    }
}
