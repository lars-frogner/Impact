//! User interface.

use crate::window::Window;
use std::sync::Arc;

/// User interface state.
#[derive(Debug)]
pub struct UserInterface {
    window: Arc<Window>,
    interaction_mode: InteractionMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InteractionMode {
    Control,
    Cursor,
}

impl UserInterface {
    /// Creates a new user interface state.
    pub fn new(window: Arc<Window>) -> Self {
        let interaction_mode = InteractionMode::cursor(&window);
        Self {
            window,
            interaction_mode,
        }
    }

    pub fn control_mode_active(&self) -> bool {
        self.interaction_mode == InteractionMode::Control
    }

    pub fn is_paused(&self) -> bool {
        self.interaction_mode == InteractionMode::Cursor
    }

    pub fn activate_control_mode(&mut self) {
        if self.interaction_mode != InteractionMode::Control {
            self.interaction_mode = InteractionMode::control(&self.window);
        }
    }

    pub fn activate_cursor_mode(&mut self) {
        if self.interaction_mode != InteractionMode::Cursor {
            self.interaction_mode = InteractionMode::cursor(&self.window);
        }
    }
}

impl InteractionMode {
    fn control(window: &Window) -> Self {
        window.set_cursor_visible(false);
        window.confine_cursor();
        Self::Control
    }

    fn cursor(window: &Window) -> Self {
        window.set_cursor_visible(true);
        window.unconfine_cursor();
        Self::Cursor
    }
}
