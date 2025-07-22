//! Commands for scene manipulation.

use crate::{command::uils::ActiveState, engine::Engine};
use anyhow::Result;
use impact_ecs::world::EntityID;
use impact_scene::skybox::Skybox;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneCommand {
    SetSkybox(Skybox),
    SetSceneEntityActiveState {
        entity_id: EntityID,
        state: ActiveState,
    },
    Clear,
}

pub fn set_skybox(engine: &Engine, skybox: Skybox) {
    impact_log::info!("Setting skybox to {skybox:?}");
    engine.scene().read().set_skybox(Some(skybox));

    engine
        .renderer()
        .read()
        .declare_render_resources_desynchronized();
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

pub fn clear(engine: &Engine) {
    impact_log::info!("Clearing scene");
    engine.reset_world();
}
