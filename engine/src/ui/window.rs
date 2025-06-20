//! User interface response to window and device events.

use crate::ui::UserInterface;
use winit::event::{DeviceEvent, WindowEvent};

/// Extension trait for [`UserInterface`]s that respond to window and device
/// events.
pub trait ResponsiveUserInterface: UserInterface {
    /// Handles the UI's reponse to a window event.
    fn handle_window_event(&self, event: &WindowEvent) -> UIEventHandlingResponse;

    /// Handles the UI's reponse to a evice input event.
    fn handle_device_event(&self, event: &DeviceEvent);
}

/// Response indicating whether a UI event was consumed or should be passed
/// through.
#[derive(Clone, Debug)]
pub struct UIEventHandlingResponse {
    /// If true, the event was consumed and should not be processed further.
    pub event_consumed: bool,
}
