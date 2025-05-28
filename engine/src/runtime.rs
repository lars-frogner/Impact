//! The top-level orchestrator of engine components.

use crate::{
    engine::{Engine, tasks::EngineTaskScheduler},
    game_loop::{GameLoop, GameLoopConfig},
    thread::ThreadPoolResult,
    ui::{UserInterface, input::UIEventHandlingResponse},
    window::{Window, WindowConfig},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::Arc,
};
use winit::{
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

/// Top-level orchestrator of engine components.
#[derive(Debug)]
pub struct Runtime {
    engine: Arc<Engine>,
    task_scheduler: EngineTaskScheduler,
    game_loop: GameLoop,
    user_interface: UserInterface,
}

pub struct RuntimeHandler {
    runtime_creator: Option<RuntimeCreator>,
    runtime: Option<Runtime>,
    window_config: WindowConfig,
}

type RuntimeCreator = Box<dyn FnOnce(Window) -> Result<Runtime>>;

/// Configuration parameters for the engine runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    n_worker_threads: NonZeroUsize,
    game_loop: GameLoopConfig,
}

/// Wrapper for an event loop controller.
#[derive(Debug)]
pub struct EventLoopController<'a>(&'a ActiveEventLoop);

impl Runtime {
    pub fn new(
        engine: Engine,
        user_interface: UserInterface,
        config: RuntimeConfig,
    ) -> Result<Self> {
        let (engine, task_scheduler) = engine.create_task_scheduler(config.n_worker_threads)?;

        let game_loop = GameLoop::new(config.game_loop);

        Ok(Self {
            engine,
            task_scheduler,
            game_loop,
            user_interface,
        })
    }

    pub fn engine(&self) -> &Engine {
        self.engine.as_ref()
    }

    pub fn arc_engine(&self) -> Arc<Engine> {
        Arc::clone(&self.engine)
    }

    fn window(&self) -> &Window {
        self.engine().window()
    }

    fn run_ui_processing(&mut self) {
        if self.engine.ui_visible() {
            // This could be moved into GameLoop::perform_iteration and the tesselation
            // could be done in parallel with other tasks. The actual running must be
            // done before beginning to execute other tasks since user interactions
            // can affect the engine state.
            let raw_ui_output = self.user_interface.run(&self.engine);
            let ui_output = self.user_interface.process_raw_output(raw_ui_output);
            *self.engine.ui_output().write().unwrap() = Some(ui_output);
        } else {
            *self.engine.ui_output().write().unwrap() = None;
        }
    }

    fn perform_game_loop_iteration(
        &mut self,
        event_loop_controller: &EventLoopController<'_>,
    ) -> ThreadPoolResult {
        self.game_loop
            .perform_iteration(&self.engine, &self.task_scheduler, event_loop_controller)
    }

    fn handle_window_event_for_ui(&mut self, event: &WindowEvent) -> UIEventHandlingResponse {
        if self.engine.ui_visible() {
            self.user_interface.handle_window_event(event)
        } else {
            UIEventHandlingResponse {
                event_consumed: false,
            }
        }
    }

    fn handle_window_event_for_engine(
        &self,
        event_loop_controller: &EventLoopController<'_>,
        event: &WindowEvent,
    ) -> Result<()> {
        self.engine
            .handle_window_event(event_loop_controller, event)
    }

    fn handle_device_event(
        &self,
        event_loop_controller: &EventLoopController<'_>,
        event: &DeviceEvent,
    ) -> Result<()> {
        self.engine
            .handle_device_event(event_loop_controller, event)
    }

    fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        self.engine.resize_rendering_surface(new_width, new_height);
    }

    fn update_pixels_per_point(&self, pixels_per_point: f64) {
        self.engine.update_pixels_per_point(pixels_per_point);
    }

    fn shutdown_requested(&self) -> bool {
        self.engine.shutdown_requested()
    }
}

impl RuntimeHandler {
    /// Creates a handler that will use the given function to
    /// create the runtime after [`Self::run`] has been called.
    pub fn new(
        create_runtime: impl FnOnce(Window) -> Result<Runtime> + 'static,
        window_config: WindowConfig,
    ) -> Self {
        Self {
            runtime_creator: Some(Box::new(create_runtime)),
            runtime: None,
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
}

impl winit::application::ApplicationHandler for RuntimeHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(runtime) = &self.runtime {
            // `runtime` is already initialized
            runtime.window().request_redraw();
            return;
        }
        match Window::create(event_loop, &self.window_config) {
            Ok(window) => {
                window.request_redraw();

                match (self
                    .runtime_creator
                    .take()
                    .expect("runtime should only be created once"))(window)
                {
                    Ok(runtime) => {
                        self.runtime = Some(runtime);
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
        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };

        if window_id != runtime.window().window().id() {
            return;
        }

        let event_loop_controller = EventLoopController(event_loop);

        let ui_handling_response = runtime.handle_window_event_for_ui(&event);

        // Do not propagate event if consumed by UI event handler
        if ui_handling_response.event_consumed {
            return;
        }

        match event {
            WindowEvent::RedrawRequested => {
                runtime.run_ui_processing();

                let result = runtime.perform_game_loop_iteration(&event_loop_controller);

                if let Err(errors) = result {
                    log::error!("Unhandled errors: {:?}", errors);
                    event_loop_controller.exit();
                } else {
                    runtime.window().request_redraw();
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

        if let Err(error) = runtime.handle_window_event_for_engine(&event_loop_controller, &event) {
            log::error!("Window event handling error: {:?}", error);
            event_loop_controller.exit();
        } else if runtime.shutdown_requested() {
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
        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };

        let event_loop_controller = EventLoopController(event_loop);

        if let Err(error) = runtime.handle_device_event(&event_loop_controller, &event) {
            log::error!("Device event handling error: {:?}", error);
            event_loop_controller.exit();
        }
    }
}

impl std::fmt::Debug for RuntimeHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.runtime.fmt(f)
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            n_worker_threads: NonZeroUsize::new(1).unwrap(),
            game_loop: GameLoopConfig::default(),
        }
    }
}

impl EventLoopController<'_> {
    /// Terminates the event loop.
    pub fn exit(&self) {
        self.0.exit();
    }
}
