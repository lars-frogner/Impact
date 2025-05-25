//! Window management.

pub mod input;

pub use winit::event::WindowEvent;

use crate::game_loop::GameLoop;
use anyhow::{Result, anyhow};
use std::{num::NonZeroU32, sync::Arc};
use winit::{
    application::ApplicationHandler as EngineHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{CursorGrabMode, Window as WinitWindow, WindowAttributes, WindowId},
};

/// Top-level entity that manages the window, event loop and game loop.
pub struct GameHandler {
    create_game_loop: Option<Box<dyn FnOnce(Window) -> Result<GameLoop>>>,
    game_loop: Option<GameLoop>,
}

/// Wrapper for a window.
#[derive(Clone, Debug)]
pub struct Window {
    window: Arc<WinitWindow>,
}

/// Wrapper for an event loop controller.
#[derive(Debug)]
pub struct EventLoopController<'a>(&'a ActiveEventLoop);

/// Calculates the ratio of width to height.
pub fn calculate_aspect_ratio(width: NonZeroU32, height: NonZeroU32) -> f32 {
    u32::from(width) as f32 / u32::from(height) as f32
}

impl GameHandler {
    /// Creates a new `GameHandler` that will use the given function to create
    /// the game loop after [`Self::run`] has been called.
    pub fn new(create_game_loop: impl FnOnce(Window) -> Result<GameLoop> + 'static) -> Self {
        Self {
            create_game_loop: Some(Box::new(create_game_loop)),
            game_loop: None,
        }
    }

    /// Creates the window and game loop and begins executing the event loop.
    pub fn run(&mut self) -> Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self)?;
        Ok(())
    }
}

impl EngineHandler for GameHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(game_loop) = &self.game_loop {
            // `game_loop` is already initialized
            game_loop.window().request_redraw();
            return;
        }
        match event_loop
            .create_window(
                WindowAttributes::default().with_inner_size(PhysicalSize::new(1600, 1200)),
            )
            .map_err(|error| error.into())
            .and_then(Window::wrap)
        {
            Ok(window) => {
                window.request_redraw();

                match (self
                    .create_game_loop
                    .take()
                    .expect("game loop should only be created once"))(window)
                {
                    Ok(game_loop) => {
                        self.game_loop = Some(game_loop);
                    }
                    Err(error) => {
                        log::error!("Game loop creation error: {:?}", error);
                        event_loop.exit();
                    }
                }
            }
            Err(error) => {
                log::error!("Window creation error: {:?}", error);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(game_loop) = self.game_loop.as_mut() else {
            return;
        };

        if window_id != game_loop.window().window().id() {
            return;
        }

        let event_loop_controller = EventLoopController(event_loop);

        match event {
            WindowEvent::RedrawRequested => {
                let result = game_loop.perform_iteration(&event_loop_controller);
                if let Err(errors) = result {
                    log::error!("Unhandled errors: {:?}", errors);
                    event_loop_controller.exit();
                } else {
                    game_loop.window().request_redraw();
                }
            }
            // Exit if user requests close
            WindowEvent::CloseRequested => event_loop_controller.exit(),
            // Resize rendering surface when window is resized
            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 {
                    log::error!("Tried resizing window to zero size");
                    event_loop_controller.exit();
                } else {
                    game_loop.resize_rendering_surface(
                        NonZeroU32::new(new_size.width).unwrap(),
                        NonZeroU32::new(new_size.height).unwrap(),
                    );
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                game_loop.update_pixels_per_point(scale_factor);
            }
            _ => {}
        }

        if let Err(error) = game_loop.handle_window_event(&event_loop_controller, &event) {
            log::error!("Window event handling error: {:?}", error);
            event_loop_controller.exit();
        } else if game_loop.shutdown_requested() {
            log::info!("Shutting down after request");
            event_loop_controller.exit();
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Some(game_loop) = self.game_loop.as_mut() else {
            return;
        };

        let event_loop_controller = EventLoopController(event_loop);

        if let Err(error) = game_loop.handle_device_event(&event_loop_controller, &event) {
            log::error!("Device event handling error: {:?}", error);
            event_loop_controller.exit();
        }
    }
}

impl std::fmt::Debug for GameHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.game_loop.fmt(f)
    }
}

impl Window {
    /// Returns the underlying [`winit::Window`].
    pub fn window(&self) -> &WinitWindow {
        &self.window
    }

    /// Returns the underlying [`winit::Window`] wrapped in an [`Arc`].
    pub fn arc_window(&self) -> Arc<WinitWindow> {
        Arc::clone(&self.window)
    }

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the window is on.
    pub fn pixels_per_point(&self) -> f64 {
        self.window.scale_factor()
    }

    /// Returns a tuple (width, height) with the extents of the
    /// window in number of pixels.
    pub fn dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        let window_size = self.window.inner_size();
        (
            NonZeroU32::new(window_size.width).unwrap(),
            NonZeroU32::new(window_size.height).unwrap(),
        )
    }

    /// Returns the ratio of width to height of the window.
    pub fn aspect_ratio(&self) -> f32 {
        let (width, height) = self.dimensions();
        calculate_aspect_ratio(width, height)
    }

    /// Modifies the cursor's visibility.
    ///
    /// If `false`, this will hide the cursor. If `true`, this will show the
    /// cursor.
    pub fn set_cursor_visible(&self, visible: bool) {
        self.window.set_cursor_visible(visible);
    }

    /// Confines the cursor to the window area.
    pub fn confine_cursor(&self) {
        self.window
            .set_cursor_grab(CursorGrabMode::Confined)
            .expect("Could not confine cursor");
    }

    /// Allows the cursor to leave the window area.
    pub fn unconfine_cursor(&self) {
        self.window
            .set_cursor_grab(CursorGrabMode::None)
            .expect("Could not unconfine cursor");
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn wrap(window: WinitWindow) -> Result<Self> {
        let window_size = window.inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            Err(anyhow!("degenerate window dimensions"))
        } else {
            Ok(Self {
                window: Arc::new(window),
            })
        }
    }
}

impl EventLoopController<'_> {
    /// Terminates the event loop.
    pub fn exit(&self) {
        self.0.exit();
    }
}
