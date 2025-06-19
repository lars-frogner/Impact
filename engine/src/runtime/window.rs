//! A handler that manages the lifecycle of a [`Window`] and [`Runtime`].

use crate::{
    runtime::Runtime,
    ui::window::UIEventHandlingResponse,
    window::{Window, WindowConfig},
};
use anyhow::Result;
use std::num::NonZeroU32;
use winit::{
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

/// A handler that manages the lifecycle of a [`Window`] and [`Runtime`],
/// coordinating between the window system and the engine runtime.
///
/// This struct acts as the main event loop coordinator, handling:
/// - [`Window`] creation and management.
/// - [`Runtime`] creation using a provided factory function.
/// - Window lifecycle events (resize, redraw, close, etc.).
/// - Graceful shutdown.
///
/// The handler follows a two-phase initialization:
/// 1. Creation with a runtime factory function and window configuration.
/// 2. Execution via [`Self::run`] which creates the window and runtime and
///    starts the event loop.
///
/// # Example
/// ```no_run
/// let mut handler = WindowRuntimeHandler::new(
///     |window| {
///         create_runtime(window, ...)
///     },
///     WindowConfig::default(),
/// );
/// handler.run()?;
/// ```
pub struct WindowRuntimeHandler {
    runtime_creator: Option<RuntimeCreator>,
    runtime_and_window: Option<(Runtime, Window)>,
    window_config: WindowConfig,
}

type RuntimeCreator = Box<dyn FnOnce(Window) -> Result<Runtime>>;

impl Runtime {
    fn handle_window_event_for_ui(&mut self, event: &WindowEvent) -> UIEventHandlingResponse {
        self.user_interface.handle_window_event(event)
    }

    fn handle_device_event_for_ui(&mut self, event: &DeviceEvent) {
        self.user_interface.handle_device_event(event);
    }

    fn handle_window_event_for_engine(&self, event: &WindowEvent) -> Result<()> {
        self.engine.handle_window_event(event)
    }

    fn handle_device_event_for_engine(&self, event: &DeviceEvent) -> Result<()> {
        self.engine.handle_device_event(event)
    }
}

impl WindowRuntimeHandler {
    /// Creates a handler that will use the given function to create the runtime
    /// after [`Self::run`] has been called.
    pub fn new(
        create_runtime: impl FnOnce(Window) -> Result<Runtime> + 'static,
        window_config: WindowConfig,
    ) -> Self {
        Self {
            runtime_creator: Some(Box::new(create_runtime)),
            runtime_and_window: None,
            window_config,
        }
    }

    /// Creates the window and runtime and begins executing the event loop.
    pub fn run(&mut self) -> Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self)?;
        Ok(())
    }

    fn window(&self) -> Option<&Window> {
        self.runtime_and_window.as_ref().map(|(_, window)| window)
    }

    fn runtime_mut(&mut self) -> Option<&mut Runtime> {
        self.runtime_and_window.as_mut().map(|(runtime, _)| runtime)
    }

    fn runtime_mut_and_window(&mut self) -> Option<(&mut Runtime, &Window)> {
        self.runtime_and_window
            .as_mut()
            .map(|(runtime, window)| (runtime, &*window))
    }
}

impl winit::application::ApplicationHandler for WindowRuntimeHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window() {
            // Window is already initialized
            window.request_redraw();
            return;
        }
        match Window::create(event_loop, &self.window_config) {
            Ok(window) => {
                window.request_redraw();

                match (self
                    .runtime_creator
                    .take()
                    .expect("runtime should only be created once"))(
                    window.clone()
                ) {
                    Ok(runtime) => {
                        self.runtime_and_window = Some((runtime, window));
                    }
                    Err(error) => {
                        log::error!("Runtime creation error: {:?}", error);
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
        let Some((runtime, window)) = self.runtime_mut_and_window() else {
            return;
        };
        if window_id != window.window().id() {
            return;
        }

        let ui_handling_response = runtime.handle_window_event_for_ui(&event);

        // Do not propagate event if consumed by UI event handler
        if ui_handling_response.event_consumed {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                runtime.run_ui_processing();

                let result = runtime.perform_game_loop_iteration();

                if let Err(errors) = result {
                    log::error!("Aborting due to fatal errors: {:?}", errors);
                    event_loop.exit();
                } else {
                    window.request_redraw();
                }
            }
            // Exit if user requests close
            WindowEvent::CloseRequested => event_loop.exit(),
            // Resize rendering surface when window is resized
            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 {
                    log::error!("Tried resizing window to zero size");
                    event_loop.exit();
                } else {
                    runtime.resize_rendering_surface(
                        NonZeroU32::new(new_size.width).unwrap(),
                        NonZeroU32::new(new_size.height).unwrap(),
                    );
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                runtime.update_pixels_per_point(scale_factor);
            }
            _ => {}
        }

        if let Err(error) = runtime.handle_window_event_for_engine(&event) {
            log::error!("Window event handling error: {:?}", error);
            event_loop.exit();
        } else if runtime.shutdown_requested() {
            log::info!("Shutting down after request");
            event_loop.exit();
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Some(runtime) = self.runtime_mut() else {
            return;
        };

        runtime.handle_device_event_for_ui(&event);

        if let Err(error) = runtime.handle_device_event_for_engine(&event) {
            log::error!("Device event handling error: {:?}", error);
            event_loop.exit();
        }
    }
}

impl std::fmt::Debug for WindowRuntimeHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.runtime_and_window.fmt(f)
    }
}
