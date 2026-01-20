//! Scene setup.

use crate::{Game, PlayerMode, command::GAME_COMMANDS, scripting, user_interface::UI_COMMANDS};
use anyhow::Result;
use impact::engine::Engine;
use roc_integration::roc;

#[roc(parents = "Game")]
#[derive(Clone, Debug)]
pub struct SetupContext {
    pub player_mode: PlayerMode,
}

impl Game {
    pub fn setup_scene(&self) -> Result<()> {
        scripting::setup_scene(self.create_setup_context())
    }

    pub fn reset_scene(&self, engine: &Engine) -> Result<()> {
        log::debug!("Resetting scene");
        engine.reset_world()?;

        GAME_COMMANDS.clear();
        UI_COMMANDS.clear();

        self.setup_scene()
    }

    fn create_setup_context(&self) -> SetupContext {
        SetupContext {
            player_mode: self.game_options.read().player_mode,
        }
    }
}
