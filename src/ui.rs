//! User interface.

use crate::window::Window;

/// User interface state.
#[derive(Debug)]
pub struct UserInterface {
    cursor_visible: bool,
}

impl UserInterface {
    /// Creates a new user interface state.
    pub fn new() -> Self {
        Self {
            cursor_visible: true,
        }
    }

    /// If the cursor is visible, hide it. If the cursor is hidden,
    /// show make it visible.
    pub fn toggle_cursor_visibility(&mut self, window: &Window) {
        self.cursor_visible = !self.cursor_visible;
        window.set_cursor_visible(self.cursor_visible);
    }
}
