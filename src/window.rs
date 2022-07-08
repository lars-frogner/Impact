mod input;

use crate::world::World;
use anyhow::Result;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window as WinitWindow, WindowBuilder},
};

pub use input::{HandlingResult, InputHandler};

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use anyhow::anyhow;
        use wasm_bindgen::prelude::*;

        const WEB_WINDOW_WIDTH: u32 = 450;
        const WEB_WINDOW_HEIGHT: u32 = 400;
        // HTML object that will be the parent of the canvas we render to
        const WEB_WINDOW_CONTAINER_ID: &str = "impact-container";
    }
}

pub struct Window {
    window: WinitWindow,
    event_loop: EventLoop<()>,
}

impl Window {
    pub fn new() -> Result<Self> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop)?;
        #[cfg(target_arch = "wasm32")]
        {
            set_window_size(&window);
            add_window_canvas_to_parent_element(&window)?;
        }
        Ok(Self { event_loop, window })
    }

    pub fn window(&self) -> &WinitWindow {
        &self.window
    }

    pub fn run_event_loop(self, input_handler: InputHandler, mut world: World) -> ! {
        let Self { window, event_loop } = self;

        event_loop.run(move |event, _, control_flow| match event {
            // Handle window events
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match input_handler.handle_event(&mut world, control_flow, &event) {
                    HandlingResult::Handled => {}
                    HandlingResult::Unhandled => {
                        match event {
                            // Exit if user requests close
                            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                            // Resize rendering surface when window is resized..
                            WindowEvent::Resized(new_size) => {
                                world
                                    .renderer_mut()
                                    .resize_surface((new_size.width, new_size.height));
                            }
                            // ..or when screen DPI changes
                            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                world
                                    .renderer_mut()
                                    .resize_surface((new_inner_size.width, new_inner_size.height));
                            }
                            _ => {}
                        }
                    }
                }
            }
            // Render when window requests redraw
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                match world.renderer_mut().render() {
                    Ok(_) => {}
                    Err(err) => match err.downcast_ref() {
                        // Recreate swap chain if lost
                        Some(wgpu::SurfaceError::Lost) => world.renderer_mut().initialize_surface(),
                        // Quit if GPU is out of memory
                        Some(wgpu::SurfaceError::OutOfMemory) => {
                            *control_flow = ControlFlow::Exit;
                        }
                        // Other errors should be resolved by the next frame, so we just log the error and continue
                        _ => log::error!("{:?}", err),
                    },
                }
            }
            // When all queued input events have been handled we can do other work
            Event::MainEventsCleared => {
                // Next redraw must be triggered manually
                window.request_redraw();
            }
            _ => {}
        });
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
