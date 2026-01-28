//! Input response.

use crate::{Game, PlayerMode, scripting};
use anyhow::Result;
use impact::input::{
    key::KeyboardEvent,
    mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
};
use roc_integration::roc;

#[roc(parents = "Game")]
#[derive(Clone, Debug)]
pub struct InputContext {
    pub player_mode: PlayerMode,
}

impl Game {
    pub(crate) fn create_input_context(&self) -> InputContext {
        InputContext {
            player_mode: self.game_options.read().player_mode,
        }
    }
}
