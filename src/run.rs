//! Running an event loop.

use crate::{
    control::{CameraOrientationController, Controllable, SemiDirectionalMotionController},
    game_loop::{GameLoop, GameLoopConfig},
    geometry::{ColorVertex, TextureVertex, TriangleMesh},
    physics::{
        AngularVelocity, AngularVelocityComp, Orientation, OrientationComp, PhysicsSimulator,
        PositionComp, SimulatorConfig, VelocityComp,
    },
    rendering::{
        fre, Assets, MaterialComp, MaterialID, MaterialLibrary, MaterialSpecification, ShaderID,
        TextureID,
    },
    scene::{CameraComp, CameraID, CameraRepository, MeshComp, MeshID, MeshRepository, Scene},
    window::InputHandler,
    window::{KeyActionMap, Window},
    world::World,
};
use std::f64::consts::PI;

use super::{
    geometry::{CameraConfiguration, Degrees, PerspectiveCamera, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, ImageTexture, RenderingSystem, Shader},
};
use anyhow::Result;
use nalgebra::{point, vector, Point3, Vector3};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub async fn run() -> Result<()> {
    init_logging()?;

    let (window, event_loop) = Window::new_window_and_event_loop()?;
    let world = init_world(window).await?;
    let input_handler = InputHandler::new(KeyActionMap::default());

    event_loop
        .run_game_loop(GameLoop::new(world, input_handler, GameLoopConfig::default()).unwrap());
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

async fn init_world(window: Window) -> Result<World> {
    let core_system = CoreRenderingSystem::new(&window).await?;

    let mut assets = Assets::new();
    let mut mesh_repository = MeshRepository::new();
    let mut camera_repository = CameraRepository::new();

    assets.shaders.insert(
        ShaderID(hash!("Test shader")),
        Shader::from_source(
            &core_system,
            include_str!("texture_shader.wgsl"),
            // include_str!("shader.wgsl"),
            "Test shader",
        ),
    );

    // let tree_texture = ImageTexture::from_path(&core_system, "assets/happy-tree.png", id!("Tree texture")?;
    assets.image_textures.insert(
        TextureID(hash!("Tree texture")),
        ImageTexture::from_bytes(
            &core_system,
            include_bytes!("../assets/happy-tree.png"),
            "Tree texture",
        )?,
    );

    mesh_repository
        .add_texture_mesh(
            MeshID(hash!("Test mesh")),
            TriangleMesh::new(VERTICES_WITH_TEXTURE.to_vec(), INDICES.to_vec()),
        )
        .unwrap();

    let vertical_field_of_view = Degrees(45.0);
    camera_repository
        .add_perspective_camera(
            CameraID(hash!("Camera")),
            PerspectiveCamera::new(
                CameraConfiguration::default(),
                window.aspect_ratio(),
                vertical_field_of_view,
                UpperExclusiveBounds::new(0.1, 100.0),
            ),
        )
        .unwrap();

    let mut material_library = MaterialLibrary::new();
    let material_spec = MaterialSpecification {
        shader_id: ShaderID(hash!("Test shader")),
        image_texture_ids: vec![TextureID(hash!("Tree texture"))],
    };
    material_library.add_material(MaterialID(hash!("Test material")), material_spec);

    let renderer = RenderingSystem::new(core_system, assets).await?;

    let simulator = PhysicsSimulator::new(SimulatorConfig::default());

    let motion_controller = SemiDirectionalMotionController::new(0.2, false);
    let orientation_controller =
        CameraOrientationController::new(Degrees(vertical_field_of_view.0 as f64), 1.0);

    let scene = Scene::new(camera_repository, mesh_repository, material_library);
    let world = World::new(
        window,
        scene,
        renderer,
        simulator,
        motion_controller,
        orientation_controller,
    );

    world
        .create_entities((
            &CameraComp::new(CameraID(hash!("Camera"))),
            &PositionComp(Point3::new(0.0, 0.0, 0.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), PI)),
            &VelocityComp(Vector3::zeros()),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
            &Controllable,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash!("Test mesh"))),
            &MaterialComp::new(MaterialID(hash!("Test material"))),
            &PositionComp(Point3::new(0.0, 0.0, 3.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
            &AngularVelocityComp(AngularVelocity::new(Vector3::z_axis(), Degrees(0.0))),
        ))
        .unwrap();

    Ok(world)
}

const VERTICES: &[ColorVertex<fre>] = &[
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

const VERTICES_WITH_TEXTURE: &[TextureVertex<fre>] = &[
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
