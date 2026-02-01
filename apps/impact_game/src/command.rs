//! Command buffering and execution.

use crate::{Game, InteractionMode};
use impact::command::queue::CommandQueue;
use roc_integration::roc;

pub static GAME_COMMANDS: GameCommandQueue = GameCommandQueue::new();

pub type GameCommandQueue = CommandQueue<GameCommand>;

#[roc(parents = "Command")]
#[derive(Clone, Debug, PartialEq)]
pub enum GameCommand {
    SetInteractionMode(InteractionMode),
    AddMassToInventory(f32),
    SetLauncherLaunchSpeed(f32),
}

impl Game {
    pub(crate) fn execute_game_commands(&mut self) {
        GAME_COMMANDS.execute_commands(|command| match command {
            GameCommand::SetInteractionMode(to) => {
                log::debug!("Setting interaction mode to {to:?}");
                self.game_options.interaction_mode = to;
            }
            GameCommand::AddMassToInventory(additional_mass) => {
                log::debug!("Adding mass {additional_mass} to inventory");
                self.player.inventory.add_mass(additional_mass);
            }
            GameCommand::SetLauncherLaunchSpeed(launch_speed) => {
                log::debug!("Setting launch speed to {launch_speed}");
                self.player.launcher.set_launch_speed(launch_speed);
            }
        });
    }
}
