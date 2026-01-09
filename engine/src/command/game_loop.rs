//! Commands for controlling the game loop.

use crate::{
    command::uils::ToActiveState,
    game_loop::{GameLoopController, GameLoopState},
};

#[derive(Clone, Debug)]
pub enum GameLoopAdminCommand {
    SetGameLoop(ToActiveState),
}

pub fn set_game_loop(game_loop_controller: &mut GameLoopController, to: ToActiveState) {
    match (to, game_loop_controller.state()) {
        (ToActiveState::Enabled, _) | (ToActiveState::Opposite, GameLoopState::Paused) => {
            game_loop_controller.set_state(GameLoopState::Running);
        }
        (ToActiveState::Disabled, _) | (ToActiveState::Opposite, GameLoopState::Running) => {
            game_loop_controller.set_state(GameLoopState::Paused);
        }
    }
}
