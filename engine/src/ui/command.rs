//! Commands for controlling the user interface.

use super::UserInterface;
use roc_codegen::roc;

#[roc(parents = "Command")]
#[derive(Clone, Debug)]
pub enum UICommand {
    SetInteractionMode(ToInteractionMode),
}

#[roc(parents = "Command")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToInteractionMode {
    Control,
    Cursor,
    Opposite,
}

impl UserInterface {
    pub fn set_interaction_mode(&mut self, to: ToInteractionMode) {
        match to {
            ToInteractionMode::Control => {
                self.activate_control_mode();
            }
            ToInteractionMode::Cursor => {
                self.activate_cursor_mode();
            }
            ToInteractionMode::Opposite => {
                if self.control_mode_active() {
                    self.activate_cursor_mode();
                } else {
                    self.activate_control_mode();
                }
            }
        }
    }
}
