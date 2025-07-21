//! Commands for controlling the game loop.

use crate::{
    command::uils::ToActiveState,
    game_loop::{GameLoopController, GameLoopState},
};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameLoopCommand {
    SetGameLoop(ToActiveState),
    PauseAfterSingleIteration,
}

pub fn set_game_loop(game_loop_controller: &mut GameLoopController, to: ToActiveState) {
    match (to, game_loop_controller.state()) {
        (ToActiveState::Enabled, _)
        | (
            ToActiveState::Opposite,
            GameLoopState::Paused | GameLoopState::PauseAfterSingleIteration,
        ) => {
            game_loop_controller.set_state(GameLoopState::Running);
        }
        (ToActiveState::Disabled, _) | (ToActiveState::Opposite, GameLoopState::Running) => {
            game_loop_controller.set_state(GameLoopState::Paused);
        }
    }
}

pub fn pause_after_single_iteration(game_loop_controller: &mut GameLoopController) {
    game_loop_controller.set_state(GameLoopState::PauseAfterSingleIteration);
}
