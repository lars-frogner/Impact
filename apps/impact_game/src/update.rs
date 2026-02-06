//! World update.

use crate::{Game, InteractionMode, entities::black_body};
use roc_integration::roc;

#[roc(parents = "Game")]
#[derive(Clone, Debug)]
pub struct UpdateContext {
    pub interaction_mode: InteractionMode,
}

impl Game {
    /// Does not run update code in scripts.
    pub(crate) fn update_world(&self) {
        black_body::update_black_bodies(self.engine());
    }

    pub(crate) fn create_update_context(&self) -> UpdateContext {
        UpdateContext {
            interaction_mode: self.game_options.interaction_mode,
        }
    }
}
