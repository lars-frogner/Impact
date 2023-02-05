//! Running an event loop.

use super::{
    geometry::{Degrees, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, ImageTexture, RenderingSystem},
};
use crate::{
    control::{
        CameraOrientationController, Controllable, RollFreeCameraOrientationController,
        SemiDirectionalMotionController,
    },
    game_loop::{GameLoop, GameLoopConfig},
    geometry::{ColorVertex, NormalVectorVertex, TextureVertex, TriangleMesh},
    physics::{
        AngularVelocity, AngularVelocityComp, Orientation, OrientationComp, PhysicsSimulator,
        PositionComp, SimulatorConfig, VelocityComp,
    },
    rendering::{fre, Assets, TextureID},
    scene::{
        BlinnPhongComp, FixedColorComp, FixedTextureComp, MeshComp, MeshID, MeshRepository,
        Omnidirectional, PerspectiveCameraComp, RadianceComp, Scene, VertexColorComp,
    },
    window::InputHandler,
    window::{KeyActionMap, Window},
    world::World,
};
use anyhow::Result;
use impact_utils::{hash32, hash64};
use nalgebra::{point, vector, Point3, UnitVector3, Vector3};
use std::f64::consts::PI;

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

    // let tree_texture = ImageTexture::from_path(&core_system, "assets/happy-tree.png", id!("Tree texture")?;
    assets.image_textures.insert(
        TextureID(hash32!("Tree texture")),
        ImageTexture::from_bytes(
            &core_system,
            include_bytes!("../assets/happy-tree.png"),
            "Tree texture",
        )?,
    );

    mesh_repository
        .add_texture_mesh(
            MeshID(hash64!("Texture mesh")),
            TriangleMesh::new(VERTICES_WITH_TEXTURE.to_vec(), INDICES.to_vec()),
        )
        .unwrap();
    mesh_repository
        .add_color_mesh(
            MeshID(hash64!("Color mesh")),
            TriangleMesh::new(VERTICES_WITH_COLOR.to_vec(), INDICES.to_vec()),
        )
        .unwrap();

    let VERTICES_WITH_NORMAL_VECTORS = &[
        NormalVectorVertex {
            position: point![-0.0868241, 0.49240386, 0.0],
            normal_vector: Vector3::z_axis(),
        },
        NormalVectorVertex {
            position: point![-0.49513406, 0.06958647, 0.0],
            normal_vector: Vector3::z_axis(),
        },
        NormalVectorVertex {
            position: point![-0.21918549, -0.44939706, 0.0],
            normal_vector: Vector3::z_axis(),
        },
        NormalVectorVertex {
            position: point![0.35966998, -0.3473291, 0.0],
            normal_vector: Vector3::z_axis(),
        },
        NormalVectorVertex {
            position: point![0.44147372, 0.2347359, 0.0],
            normal_vector: Vector3::z_axis(),
        },
    ];
    mesh_repository
        .add_normal_vector_mesh(
            MeshID(hash64!("Normal vector mesh")),
            TriangleMesh::new(VERTICES_WITH_NORMAL_VECTORS.to_vec(), INDICES.to_vec()),
        )
        .unwrap();

    let vertical_field_of_view = Degrees(45.0);
    let renderer = RenderingSystem::new(core_system, assets).await?;

    let simulator = PhysicsSimulator::new(SimulatorConfig::default());

    let motion_controller = SemiDirectionalMotionController::new(0.2, false);
    let orientation_controller =
        RollFreeCameraOrientationController::new(Degrees(f64::from(vertical_field_of_view.0)), 1.0);

    let scene = Scene::new(mesh_repository);
    let world = World::new(
        window,
        scene,
        renderer,
        simulator,
        Some(Box::new(motion_controller)),
        Some(Box::new(orientation_controller)),
    );

    world
        .create_entities((
            &PerspectiveCameraComp::new(
                vertical_field_of_view,
                UpperExclusiveBounds::new(0.1, 100.0),
            ),
            &PositionComp(Point3::new(0.0, 0.0, 0.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), PI)),
            &VelocityComp(Vector3::zeros()),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
            &Controllable,
        ))
        .unwrap();

    world
        .create_entities((
            // &MeshComp::new(MeshID(hash64!("Texture mesh"))),
            &MeshComp::new(MeshID(hash64!("Color mesh"))),
            &PositionComp(Point3::new(0.0, 0.0, 3.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
            &AngularVelocityComp(AngularVelocity::new(Vector3::z_axis(), Degrees(0.0))),
            // &FixedTextureComp(TextureID(hash32!("Tree texture"))),
            // &FixedColorComp(vector![1.0, 1.0, 1.0, 1.0]),
            &VertexColorComp,
            // &BlinnPhongComp {
            //     ambient: vector![1.0, 1.0, 1.0],
            //     diffuse: vector![1.0, 1.0, 1.0],
            //     specular: vector![1.0, 1.0, 1.0],
            //     shininess: 1.0,
            //     alpha: 1.0,
            // },
        ))
        .unwrap();

    // world
    //     .create_entities((
    //         &PositionComp(Point3::new(0.0, 0.0, 3.0)),
    //         &RadianceComp(vector![1.0, 1.0, 1.0]),
    //         &Omnidirectional,
    //     ))
    //     .unwrap();

    Ok(world)
}

const VERTICES_WITH_COLOR: &[ColorVertex<fre>] = &[
    ColorVertex {
        position: point![-0.0868241, 0.49240386, 0.0],
        color: vector![1.0, 0.0, 0.0, 1.0],
    },
    ColorVertex {
        position: point![-0.49513406, 0.06958647, 0.0],
        color: vector![0.0, 1.0, 0.0, 1.0],
    },
    ColorVertex {
        position: point![-0.21918549, -0.44939706, 0.0],
        color: vector![0.0, 0.0, 1.0, 1.0],
    },
    ColorVertex {
        position: point![0.35966998, -0.3473291, 0.0],
        color: vector![0.0, 1.0, 1.0, 1.0],
    },
    ColorVertex {
        position: point![0.44147372, 0.2347359, 0.0],
        color: vector![1.0, 1.0, 0.0, 1.0],
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

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
