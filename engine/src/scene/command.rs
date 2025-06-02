//! Commands for scene manipulation.

use super::Scene;
use crate::{command::ActiveState, skybox::Skybox};
use impact_ecs::world::EntityID;
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
}

impl Scene {
    pub fn set_skybox(&self, skybox: Skybox) {
        self.skybox.write().unwrap().replace(skybox);
    }
}
