//! Commands for scene manipulation.

use super::Scene;
use crate::{engine::command::ActiveState, skybox::Skybox};
use impact_ecs::world::Entity;
use roc_codegen::roc;

#[roc(prefix = "Engine")]
#[derive(Clone, Debug)]
pub enum SceneCommand {
    SetSkybox(Skybox),
    SetSceneEntityActiveState { entity: Entity, state: ActiveState },
}

impl Scene {
    pub fn set_skybox(&self, skybox: Skybox) {
        self.skybox.write().unwrap().replace(skybox);
    }
}
