//! Commands for controlling the user interface.

use super::UserInterface;
use impact::{
    command::{
        AdminCommand, controller::ControlAdminCommand, queue::CommandQueue, uils::ToActiveState,
    },
    egui::FullOutput,
    engine::Engine,
    ui,
};
use roc_integration::roc;

pub type UICommandQueue = CommandQueue<UICommand>;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UICommand {
    SetInteractivity(ToActiveState),
}

impl UserInterface {
    pub(super) fn execute_commands(
        &mut self,
        output: &mut FullOutput,
        engine: &Engine,
        command_queue: &UICommandQueue,
    ) {
        command_queue.execute_commands(|command| match command {
            UICommand::SetInteractivity(to) => {
                if to.set(&mut self.config.interactive).changed {
                    if self.config.interactive {
                        ui::egui::unconfine_cursor(output);
                    } else if !self.config.disable_cursor_capture {
                        ui::egui::confine_cursor(output);
                    }
                    engine.enqueue_admin_command(AdminCommand::Control(
                        ControlAdminCommand::SetControls(ToActiveState::from_enabled(
                            !self.config.interactive,
                        )),
                    ));
                }
            }
        });
    }
}
