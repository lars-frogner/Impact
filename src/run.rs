//! Running an event loop.

use crate::{
    control::{NoMotionController, SemiDirectionalMotionController},
    game_loop::{GameLoop, GameLoopConfig},
    geometry::{
        ColorVertex, GeometricalData, Mesh, MeshInstance, MeshInstanceGroup, TextureVertex,
    },
    rendering::{Assets, RenderPassSpecification},
    window::InputHandler,
    window::Window,
    world::World,
};

use super::{
    geometry::{CameraConfiguration, Degrees, PerspectiveCamera, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, ImageTexture, RenderingSystem, Shader},
};
use anyhow::Result;
use nalgebra::{point, vector, Point3, Rotation3, Translation3, Vector3};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub async fn run() -> Result<()> {
    init_logging()?;

    let window = Window::new()?;
    let world = init_world(&window).await?;
    let input_handler = InputHandler::default();

    window.run_game_loop(GameLoop::new(world, input_handler, GameLoopConfig::default()).unwrap());
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn run_wasm() {
    if let Err(err) = run().await {
        log::error!("{}", err)
    }
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

async fn init_world(window: &Window) -> Result<World> {
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

    let mut geometrical_data = GeometricalData::new();

    geometrical_data.texture_meshes.insert(
        "Test mesh".to_string(),
        Mesh::new(VERTICES_WITH_TEXTURE.to_vec(), INDICES.to_vec()),
    );

    geometrical_data.mesh_instance_groups.insert(
        "Test mesh instance".to_string(),
        MeshInstanceGroup::new(
            vec![
                Translation3::<f32>::new(-0.5, 0.0, 0.0).into(),
                Translation3::<f32>::new(0.5, 0.0, -1.0).into(),
            ]
            .into_iter()
            .map(MeshInstance::with_transform)
            .collect(),
        ),
    );

    geometrical_data.perspective_cameras.insert(
        "Camera".to_string(),
        PerspectiveCamera::new(
            CameraConfiguration::new_looking_at(
                point![0.0, 0.0, 2.0],
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
        .with_mesh_instances(Some("Test mesh instance".to_string()))
        .with_camera(Some("Camera".to_string()));

    let renderer =
        RenderingSystem::new(core_system, assets, vec![render_pass], &geometrical_data).await?;

    let controller = SemiDirectionalMotionController::new(Rotation3::identity(), 1.0);

    Ok(World::new(geometrical_data, renderer, controller))
}

const VERTICES: &[ColorVertex<f32>] = &[
    ColorVertex {
        position: point![-0.0868241, 0.49240386, 0.0],
        color: vector![1.0, 0.0, 0.0],
    },
    ColorVertex {
        position: point![-0.49513406, 0.06958647, 0.0],
        color: vector![0.0, 1.0, 0.0],
    },
    ColorVertex {
        position: point![-0.21918549, -0.44939706, 0.0],
        color: vector![0.0, 0.0, 1.0],
    },
    ColorVertex {
        position: point![0.35966998, -0.3473291, 0.0],
        color: vector![0.0, 1.0, 1.0],
    },
    ColorVertex {
        position: point![0.44147372, 0.2347359, 0.0],
        color: vector![1.0, 1.0, 0.0],
    },
];

const VERTICES_WITH_TEXTURE: &[TextureVertex<f32>] = &[
    TextureVertex {
        position: point![-0.0868241, 0.49240386, 0.0],
        texture_coords: vector![0.4131759, 1.0 - 0.99240386],
    },
    TextureVertex {
        position: point![-0.49513406, 0.06958647, 0.0],
        texture_coords: vector![0.0048659444, 1.0 - 0.56958647],
    },
    TextureVertex {
        position: point![-0.21918549, -0.44939706, 0.0],
        texture_coords: vector![0.28081453, 1.0 - 0.05060294],
    },
    TextureVertex {
        position: point![0.35966998, -0.3473291, 0.0],
        texture_coords: vector![0.85967, 1.0 - 0.1526709],
    },
    TextureVertex {
        position: point![0.44147372, 0.2347359, 0.0],
        texture_coords: vector![0.9414737, 1.0 - 0.7347359],
    },
];

// const INDICES: &[u16] = &[0, 1, 2];
const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
