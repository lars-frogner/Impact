//! Scene setup.

use crate::{Game, PlayerMode, command::GAME_COMMANDS, user_interface::UI_COMMANDS};
use anyhow::Result;
use roc_integration::roc;

#[roc(parents = "Game")]
#[derive(Clone, Debug)]
pub struct SetupContext {
    pub player_mode: PlayerMode,
}

impl Game {
    pub(crate) fn reset_world(&self) -> Result<()> {
        self.engine().reset_world()?;

        GAME_COMMANDS.clear();
        UI_COMMANDS.clear();

        Ok(())
    }

    pub(crate) fn create_setup_context(&self) -> SetupContext {
        SetupContext {
            player_mode: self.game_options.read().player_mode,
        }
    }
}
