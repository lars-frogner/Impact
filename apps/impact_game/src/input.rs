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
    pub fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        log::trace!("Handling keyboard event {event:?}");
        scripting::handle_keyboard_event(self.create_input_context(), event)
    }

    pub fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()> {
        log::trace!("Handling mouse button event {event:?}");
        scripting::handle_mouse_button_event(self.create_input_context(), event)
    }

    pub fn handle_mouse_drag_event(&self, event: MouseDragEvent) -> Result<()> {
        log::trace!("Handling mouse drag event {event:?}");
        scripting::handle_mouse_drag_event(self.create_input_context(), event)
    }

    pub fn handle_mouse_scroll_event(&self, event: MouseScrollEvent) -> Result<()> {
        log::trace!("Handling mouse scroll event {event:?}");
        scripting::handle_mouse_scroll_event(self.create_input_context(), event)
    }

    fn create_input_context(&self) -> InputContext {
        InputContext {
            player_mode: self.game_options.read().player_mode,
        }
    }
}
