//! World update.

use crate::{Game, InteractionMode};
use roc_integration::roc;

#[roc(parents = "Game")]
#[derive(Clone, Debug)]
pub struct UpdateContext {
    pub interaction_mode: InteractionMode,
}

impl Game {
    pub(crate) fn create_update_context(&self) -> UpdateContext {
        UpdateContext {
            interaction_mode: self.game_options.interaction_mode,
        }
    }
}
