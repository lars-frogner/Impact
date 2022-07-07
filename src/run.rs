//! Running an event loop.

use crate::{
    geometry::{ColorVertex, Mesh, TextureVertex, WorldData},
    rendering::{Assets, RenderPassSpecification},
};

use super::{
    geometry::{CameraConfiguration, Degrees, PerspectiveCamera, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, ImageTexture, RenderingSystem, Shader},
};
use anyhow::Result;
use nalgebra::{Point3, Vector3};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

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

pub async fn run() -> Result<()> {
    init_logging()?;

    let event_loop = EventLoop::new();
    let window = init_window(&event_loop)?;
    let renderer = init_renderer(&window).await?;
    run_event_loop(event_loop, window, renderer);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn run_wasm() {
    if let Err(err) = run().await {
        log::error!("{}", err)
    }
}

async fn init_renderer(window: &Window) -> Result<RenderingSystem> {
    let core_system = CoreRenderingSystem::new(window).await?;

    let mut assets = Assets::new();

    assets.shaders.insert(
        "Test shader".to_string(),
        Shader::from_source(
            &core_system,
            include_str!("texture_shader.wgsl"),
            // include_str!("shader.wgsl"),
            "Test shader",
        ),
    );

    // let tree_texture = ImageTexture::from_path(&core_system, "assets/happy-tree.png", "Tree texture")?;
    assets.image_textures.insert(
        "Tree texture".to_string(),
        ImageTexture::from_bytes(
            &core_system,
            include_bytes!("../assets/happy-tree.png"),
            "Tree texture",
        )?,
    );

    let mut world = WorldData::new();

    world.texture_meshes.insert(
        "Test mesh".to_string(),
        Mesh::new(VERTICES_WITH_TEXTURE.to_vec(), INDICES.to_vec()),
    );

    world.perspective_cameras.insert(
        "Camera".to_string(),
        PerspectiveCamera::new(
            CameraConfiguration::new_looking_at(
                Point3::new(0.0, 0.0, 2.0),
                Point3::origin(),
                Vector3::y_axis(),
            ),
            core_system.surface_aspect_ratio(),
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        ),
    );

    let render_pass = RenderPassSpecification::new("Test".to_string())
        .with_clear_color(Some(wgpu::Color::BLACK))
        .with_shader(Some("Test shader".to_string()))
        .add_image_texture("Tree texture".to_string())
        .with_mesh(Some("Test mesh".to_string()))
        .with_camera(Some("Camera".to_string()));

    RenderingSystem::new(core_system, assets, vec![render_pass], &world).await
}

fn init_logging() -> Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            init_logging_web()
        } else {
            init_logging_native()
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn init_logging_web() -> Result<()> {
    // Make errors display in console
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    // Send logs to console
    console_log::init_with_level(log::Level::Warn)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn init_logging_native() -> Result<()> {
    env_logger::init();
    Ok(())
}

fn init_window(event_loop: &EventLoop<()>) -> Result<Window> {
    let window = WindowBuilder::new().build(event_loop)?;
    #[cfg(target_arch = "wasm32")]
    {
        set_window_size(&window);
        add_window_canvas_to_parent_element(&window)?;
    }
    Ok(window)
}

#[cfg(target_arch = "wasm32")]
fn set_window_size(window: &Window) {
    // Size of rendering window must be specified here rather than through CSS
    use winit::dpi::PhysicalSize;
    window.set_inner_size(PhysicalSize::new(WEB_WINDOW_WIDTH, WEB_WINDOW_HEIGHT));
}

#[cfg(target_arch = "wasm32")]
fn add_window_canvas_to_parent_element(window: &Window) -> Result<()> {
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

fn run_event_loop(event_loop: EventLoop<()>, window: Window, mut renderer: RenderingSystem) -> ! {
    event_loop.run(move |event, _, control_flow| match event {
        // Handle window events
        Event::WindowEvent {
            event: ref window_event,
            window_id,
        } if window_id == window.id() => {
            // Send event to rendering system
            if renderer.handle_input_event(window_event) {
                // If allowed by the rendering system we handle certain events here
                match window_event {
                    // Exit if user requests close or presses Escape
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    // Resize rendering surface when window is resized..
                    WindowEvent::Resized(new_size) => {
                        renderer.resize_surface((new_size.width, new_size.height));
                    }
                    // ..or when screen DPI changes
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        renderer.resize_surface((new_inner_size.width, new_inner_size.height));
                    }
                    _ => {}
                }
            }
        }
        // Render when window requests redraw
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            match renderer.render() {
                Ok(_) => {}
                Err(err) => match err.downcast_ref() {
                    // Recreate swap chain if lost
                    Some(wgpu::SurfaceError::Lost) => renderer.initialize_surface(),
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

const VERTICES: &[ColorVertex] = &[
    ColorVertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    ColorVertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    ColorVertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.0, 0.0, 1.0],
    },
    ColorVertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.0, 1.0, 1.0],
    },
    ColorVertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [1.0, 1.0, 0.0],
    },
];

const VERTICES_WITH_TEXTURE: &[TextureVertex] = &[
    TextureVertex {
        position: [-0.0868241, 0.49240386, 0.0],
        texture_coords: [0.4131759, 1.0 - 0.99240386],
    },
    TextureVertex {
        position: [-0.49513406, 0.06958647, 0.0],
        texture_coords: [0.0048659444, 1.0 - 0.56958647],
    },
    TextureVertex {
        position: [-0.21918549, -0.44939706, 0.0],
        texture_coords: [0.28081453, 1.0 - 0.05060294],
    },
    TextureVertex {
        position: [0.35966998, -0.3473291, 0.0],
        texture_coords: [0.85967, 1.0 - 0.1526709],
    },
    TextureVertex {
        position: [0.44147372, 0.2347359, 0.0],
        texture_coords: [0.9414737, 1.0 - 0.7347359],
    },
];

// const INDICES: &[u16] = &[0, 1, 2];
const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
