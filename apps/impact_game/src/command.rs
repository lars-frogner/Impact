//! Command buffering and execution.

use crate::{Game, PlayerMode};
use impact::command::queue::CommandQueue;
use roc_integration::roc;

pub static GAME_COMMANDS: GameCommandQueue = GameCommandQueue::new();

pub type GameCommandQueue = CommandQueue<GameCommand>;

#[roc(parents = "Command")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameCommand {
    SetPlayerMode(PlayerMode),
}

impl Game {
    pub fn execute_game_commands(&self) {
        GAME_COMMANDS.execute_commands(|command| match command {
            GameCommand::SetPlayerMode(to) => {
                log::info!("Setting player mode to {to:?}");
                self.game_options.write().player_mode = to;
            }
        });
    }
}
