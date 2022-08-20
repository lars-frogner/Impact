//! Window management.

mod input;

pub use input::{HandlingResult, InputHandler};
pub use winit::event::WindowEvent;

use crate::game_loop::GameLoop;
use anyhow::Result;
use winit::{
    event::Event,
    event_loop::{ControlFlow as WinitControlFlow, EventLoop},
    window::{Window as WinitWindow, WindowBuilder},
};

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use anyhow::anyhow;

        const WEB_WINDOW_WIDTH: u32 = 450;
        const WEB_WINDOW_HEIGHT: u32 = 400;
        // HTML object that will be the parent of the canvas we render to
        const WEB_WINDOW_CONTAINER_ID: &str = "impact-container";
    }
}

/// Wrapper for a window with an associated event loop.
#[derive(Debug)]
pub struct Window {
    window: WinitWindow,
    event_loop: EventLoop<()>,
}

/// Wrapper for an event loop control flow.
#[derive(Debug)]
pub struct ControlFlow<'a>(&'a mut WinitControlFlow);

/// Calculates the ratio of width to height.
pub fn calculate_aspect_ratio(width: u32, height: u32) -> f32 {
    width as f32 / height as f32
}

impl Window {
    /// Creates a new window with an associated event loop.
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop)?;

        #[cfg(target_arch = "wasm32")]
        {
            // For wasm we need to set the window size manually
            // and add the window to the DOM
            set_window_size(&window);
            add_window_canvas_to_parent_element(&window)?;
        }

        Ok(Self { event_loop, window })
    }

    /// Returns the underlying [`winit::Window`].
    pub fn window(&self) -> &WinitWindow {
        &self.window
    }

    /// Returns the ratio of width to height of the window.
    pub fn aspect_ratio(&self) -> f32 {
        let window_size = self.window().inner_size();
        calculate_aspect_ratio(window_size.width, window_size.height)
    }

    /// Wraps the given game loop in an event loop that can capture
    /// window events and runs the loop.
    pub fn run_game_loop(self, mut game_loop: GameLoop) -> ! {
        let Self { window, event_loop } = self;
        event_loop.run(move |event, _, control_flow| {
            let mut control_flow = ControlFlow(control_flow);
            match event {
                // Handle window events
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match game_loop.handle_input_event(&mut control_flow, &event) {
                        HandlingResult::Handled => {}
                        HandlingResult::Unhandled => {
                            match event {
                                // Exit if user requests close
                                WindowEvent::CloseRequested => control_flow.exit(),
                                // Resize rendering surface when window is resized..
                                WindowEvent::Resized(new_size) => {
                                    game_loop.resize_rendering_surface((
                                        new_size.width,
                                        new_size.height,
                                    ));
                                }
                                // ..or when screen DPI changes
                                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                    game_loop.resize_rendering_surface((
                                        new_inner_size.width,
                                        new_inner_size.height,
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // When all queued input events have been handled we can do other work
                Event::MainEventsCleared => {
                    let result = game_loop.perform_iteration(&mut control_flow);
                    if let Err(errors) = result {
                        log::error!("Unhandled errors: {:?}", errors);
                        control_flow.exit();
                    }
                }
                _ => {}
            }
        });
    }
}

impl<'a> ControlFlow<'a> {
    /// Terminates the event loop.
    pub fn exit(&mut self) {
        *self.0 = WinitControlFlow::Exit;
    }
}

#[cfg(target_arch = "wasm32")]
fn set_window_size(window: &WinitWindow) {
    // Size of rendering window must be specified here rather than through CSS
    use winit::dpi::PhysicalSize;
    window.set_inner_size(PhysicalSize::new(WEB_WINDOW_WIDTH, WEB_WINDOW_HEIGHT));
}

#[cfg(target_arch = "wasm32")]
fn add_window_canvas_to_parent_element(window: &WinitWindow) -> Result<()> {
    use winit::platform::web::WindowExtWebSys;
    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let canvas = web_sys::Element::from(window.canvas());
            let container = doc.get_element_by_id(WEB_WINDOW_CONTAINER_ID)?;
            container.append_child(&canvas).ok()?;
            Some(())
        })
        .ok_or_else(|| anyhow!("Could not get window object"))
}
