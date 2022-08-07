//! Running an event loop.

use crate::{
    control::{NoMotionController, SemiDirectionalMotionController},
    game_loop::{GameLoop, GameLoopConfig},
    geometry::{ColorVertex, GeometricalData, Mesh, TextureVertex},
    rendering::{Assets, MaterialLibrary, MaterialSpecification, ModelLibrary, ModelSpecification},
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
        hash!("Test shader"),
        Shader::from_source(
            &core_system,
            include_str!("texture_shader.wgsl"),
            // include_str!("shader.wgsl"),
            "Test shader",
        ),
    );

    // let tree_texture = ImageTexture::from_path(&core_system, "assets/happy-tree.png", id!("Tree texture")?;
    assets.image_textures.insert(
        hash!("Tree texture"),
        ImageTexture::from_bytes(
            &core_system,
            include_bytes!("../assets/happy-tree.png"),
            "Tree texture",
        )?,
    );

    let mut geometrical_data = GeometricalData::new();

    geometrical_data.add_texture_mesh(
        hash!("Test mesh"),
        Mesh::new(VERTICES_WITH_TEXTURE.to_vec(), INDICES.to_vec()),
    );

    geometrical_data.add_perspective_camera(
        hash!("Camera"),
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

    let mut material_library = MaterialLibrary::new();
    let material_spec = MaterialSpecification {
        shader_id: hash!("Test shader"),
        image_texture_ids: vec![hash!("Tree texture")],
    };
    material_library.add_material(hash!("Test material"), material_spec);

    let mut model_library = ModelLibrary::new(material_library);
    let model_spec = ModelSpecification {
        material_id: hash!("Test material"),
        mesh_id: hash!("Test mesh"),
    };
    model_library.add_model(hash!("Test model"), model_spec);

    let camera = hash!("Camera");
    let models = vec![hash!("Test model")];

    let renderer = RenderingSystem::new(
        core_system,
        assets,
        model_library,
        &geometrical_data,
        camera,
        models,
        wgpu::Color::GREEN,
    )
    .await?;

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
