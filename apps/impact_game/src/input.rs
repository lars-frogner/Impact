//! Input response.

use crate::{Game, InteractionMode};
use roc_integration::roc;

#[roc(parents = "Game")]
#[derive(Clone, Debug)]
pub struct InputContext {
    pub interaction_mode: InteractionMode,
}

impl Game {
    pub(crate) fn create_input_context(&self) -> InputContext {
        InputContext {
            interaction_mode: self.game_options.interaction_mode,
        }
    }
}
