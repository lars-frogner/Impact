//! Commands for scene manipulation.

use crate::{command::uils::ActiveState, engine::Engine, lock_order::OrderedRwLock};
use anyhow::Result;
use impact_camera::CameraID;
use impact_id::EntityID;
use impact_physics::medium::UniformMedium;
use impact_scene::skybox::Skybox;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq)]
pub enum SceneCommand {
    SetActiveCamera {
        entity_id: EntityID,
    },
    SetSkybox(Skybox),
    SetMedium(UniformMedium),
    SetSceneEntityActiveState {
        entity_id: EntityID,
        state: ActiveState,
    },
}

pub fn set_active_camera(engine: &Engine, entity_id: EntityID) -> Result<()> {
    log::info!("Setting camera of entity {entity_id} to active");

    let scene = engine.scene().oread();
    let mut camera_manager = scene.camera_manager().owrite();

    camera_manager.set_active_camera(CameraID::from_entity_id(entity_id))
}

pub fn set_skybox(engine: &Engine, skybox: Skybox) {
    log::info!("Setting skybox to {}", skybox.cubemap_texture_id());
    engine.scene().oread().set_skybox(Some(skybox));
}

pub fn set_medium(engine: &Engine, medium: UniformMedium) {
    log::info!("Setting medium to {medium:?}");
    engine.simulator().owrite().set_medium(medium);
}

pub fn set_scene_entity_active_state(
    engine: &Engine,
    entity_id: EntityID,
    state: ActiveState,
) -> Result<()> {
    log::debug!("Setting state of scene entity with ID {entity_id} to {state:?}");
    match state {
        ActiveState::Enabled => engine.enable_scene_entity(entity_id),
        ActiveState::Disabled => engine.disable_scene_entity(entity_id),
    }
}
