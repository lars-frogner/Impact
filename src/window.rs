//! Window management.

mod input;

pub use input::{HandlingResult, InputHandler, KeyActionMap, MouseInputHandler};
pub use winit::event::WindowEvent;

use crate::game_loop::GameLoop;
use anyhow::Result;
use winit::{
    dpi::PhysicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop as WinitEventLoop, EventLoopWindowTarget},
    window::{Window as WinitWindow, WindowBuilder},
};

/// Wrapper for a window.
#[derive(Debug)]
pub struct Window {
    window: WinitWindow,
}

/// Wrapper for an event loop.
#[derive(Debug)]
pub struct EventLoop {
    event_loop: WinitEventLoop<()>,
}

/// Wrapper for an event loop controller.
#[derive(Debug)]
pub struct EventLoopController<'a>(&'a EventLoopWindowTarget<()>);

/// Calculates the ratio of width to height.
pub fn calculate_aspect_ratio(width: u32, height: u32) -> f32 {
    width as f32 / height as f32
}

impl Window {
    /// Creates a new window with an associated event loop.
    pub fn new_window_and_event_loop() -> Result<(Self, EventLoop)> {
        let event_loop = WinitEventLoop::new()?;
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(1600, 1200))
            .build(&event_loop)?;

        event_loop.set_control_flow(ControlFlow::Poll);

        Ok((Self::wrap(window), EventLoop::wrap(event_loop)))
    }

    /// Returns the underlying [`winit::Window`].
    pub fn window(&self) -> &WinitWindow {
        &self.window
    }

    /// Returns a tuple (width, height) with the extents of the
    /// window in number of pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        let window_size = self.window().inner_size();
        (window_size.width, window_size.height)
    }

    /// Returns the ratio of width to height of the window.
    pub fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.dimensions();
        calculate_aspect_ratio(width, height)
    }

    /// Modifies the cursor's visibility.
    ///
    /// If `false`, this will hide the cursor. If `true`, this will show the cursor.
    pub fn set_cursor_visible(&self, visible: bool) {
        self.window.set_cursor_visible(visible);
    }

    fn wrap(window: WinitWindow) -> Self {
        Self { window }
    }
}

impl EventLoop {
    /// Wraps the given game loop in an event loop that can capture
    /// window events and runs the loop.
    pub fn run_game_loop(self, mut game_loop: GameLoop) -> Result<()> {
        let event_loop = self.unwrap();
        event_loop.run(move |event, event_loop_window_target| {
            let event_loop_controller = EventLoopController(event_loop_window_target);
            match event {
                // Handle window events
                Event::WindowEvent { event, window_id }
                    if window_id == game_loop.world().window().window().id() =>
                {
                    match game_loop.handle_window_event(&event_loop_controller, &event) {
                        Ok(HandlingResult::Handled) => {}
                        Ok(HandlingResult::Unhandled) => {
                            match event {
                                // Exit if user requests close
                                WindowEvent::CloseRequested => event_loop_controller.exit(),
                                // Resize rendering surface when window is resized..
                                WindowEvent::Resized(new_size) => {
                                    game_loop.resize_rendering_surface((
                                        new_size.width,
                                        new_size.height,
                                    ));
                                }
                                _ => {}
                            }
                        }
                        Err(error) => {
                            log::error!("Window event handling error: {:?}", error);
                            event_loop_controller.exit();
                        }
                    }
                }
                Event::DeviceEvent { event, .. } => {
                    match game_loop.handle_device_event(&event_loop_controller, &event) {
                        Ok(_) => {}
                        Err(error) => {
                            log::error!("Device event handling error: {:?}", error);
                            event_loop_controller.exit();
                        }
                    }
                }
                // When all queued input events have been handled we can do other work
                Event::AboutToWait => {
                    let result = game_loop.perform_iteration(&event_loop_controller);
                    if let Err(errors) = result {
                        log::error!("Unhandled errors: {:?}", errors);
                        event_loop_controller.exit();
                    }
                }
                _ => {}
            }
        })?;
        Ok(())
    }

    fn wrap(event_loop: WinitEventLoop<()>) -> Self {
        Self { event_loop }
    }

    fn unwrap(self) -> WinitEventLoop<()> {
        let Self { event_loop } = self;
        event_loop
    }
}

impl<'a> EventLoopController<'a> {
    /// Terminates the event loop.
    pub fn exit(&self) {
        self.0.exit();
    }
}
