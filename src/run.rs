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
    geometry::TriangleMesh,
    physics::{
        AngularVelocity, AngularVelocityComp, Orientation, OrientationComp, PhysicsSimulator,
        PositionComp, SimulatorConfig, VelocityComp,
    },
    rendering::{Assets, TextureID},
    scene::{
        BlinnPhongComp, DiffuseTexturedBlinnPhongComp, DirectionComp, FixedColorComp,
        FixedTextureComp, LightDirection, LightSpaceDepthComp, MeshComp, MeshID, MeshRepository,
        Omnidirectional, PerspectiveCameraComp, RadianceComp, ScalingComp, Scene, VertexColorComp,
    },
    window::InputHandler,
    window::{KeyActionMap, Window},
    world::World,
};
use anyhow::Result;
use impact_utils::{hash32, hash64};
use nalgebra::{vector, Point3, Vector3};
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
        .add_mesh(
            MeshID(hash64!("Plane mesh")),
            TriangleMesh::create_plane(1.0, 1.0),
        )
        .unwrap();

    mesh_repository
        .add_mesh(
            MeshID(hash64!("Box mesh")),
            TriangleMesh::create_box(1.0, 1.0, 1.0),
        )
        .unwrap();

    mesh_repository
        .add_mesh(
            MeshID(hash64!("Cylinder mesh")),
            TriangleMesh::create_cylinder(1.0, 1.0, 100),
        )
        .unwrap();

    mesh_repository
        .add_mesh(
            MeshID(hash64!("Sphere mesh")),
            TriangleMesh::create_sphere(100),
        )
        .unwrap();

    let vertical_field_of_view = Degrees(45.0);
    let renderer = RenderingSystem::new(core_system, assets).await?;

    let simulator = PhysicsSimulator::new(SimulatorConfig::default());

    let motion_controller = SemiDirectionalMotionController::new(0.2, true);
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

    for model_comps in world.load_models_from_obj_file("assets/bunny.obj").unwrap() {
        world
            .create_entities(
                model_comps
                    .combined_with((
                        &PositionComp(Point3::new(-2.0, 1.0, 10.0)),
                        &ScalingComp(1.0),
                        &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
                        &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(2.0))),
                        // &FixedColorComp(vector![0.5, 0.5, 0.5]),
                        &BlinnPhongComp {
                            diffuse: vector![0.3, 0.3, 0.5],
                            specular: vector![0.6, 0.6, 0.6],
                            shininess: 13.0,
                        },
                        // &LightSpaceDepthComp,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }

    for model_comps in world
        .load_models_from_obj_file("assets/teapot.obj")
        .unwrap()
    {
        world
            .create_entities(
                model_comps
                    .combined_with((
                        &PositionComp(Point3::new(2.0, 1.0, 8.0)),
                        &ScalingComp(0.1),
                        &OrientationComp(Orientation::from_axis_angle(
                            &Vector3::x_axis(),
                            -PI / 2.0,
                        )),
                        &AngularVelocityComp(AngularVelocity::new(
                            Vector3::y_axis(),
                            Degrees(-2.0),
                        )),
                        // &FixedColorComp(vector![0.5, 0.5, 0.5]),
                        &BlinnPhongComp {
                            diffuse: vector![0.6, 0.2, 0.5],
                            specular: vector![0.4, 0.4, 0.4],
                            shininess: 4.0,
                        },
                        // &LightSpaceDepthComp,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Cylinder mesh"))),
            &PositionComp(Point3::new(0.0, 4.0, 6.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::z_axis(), 0.4)),
            &AngularVelocityComp(AngularVelocity::new(Vector3::x_axis(), Degrees(3.0))),
            &ScalingComp(1.5),
            &BlinnPhongComp {
                diffuse: vector![0.2, 0.5, 0.8],
                specular: vector![0.9, 0.9, 0.9],
                shininess: 9.0,
            },
        ))
        .unwrap();

    world
        .create_entities((
            &PerspectiveCameraComp::new(
                vertical_field_of_view,
                UpperExclusiveBounds::new(0.1, 100.0),
            ),
            &PositionComp(Point3::new(0.0, 1.0, 0.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), PI)),
            &VelocityComp(Vector3::zeros()),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
            &Controllable,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Plane mesh"))),
            &PositionComp(Point3::new(0.0, 0.0, 10.0)),
            &ScalingComp(20.0),
            // &FixedTextureComp(TextureID(hash32!("Tree texture"))),
            // &FixedColorComp(vector![1.0, 1.0, 1.0, 1.0]),
            &DiffuseTexturedBlinnPhongComp {
                diffuse: TextureID(hash32!("Tree texture")), //vector![0.4, 0.4, 0.4],
                specular: vector![0.3, 0.3, 0.3],
                shininess: 6.0,
            },
            // &LightSpaceDepthComp,
        ))
        .unwrap();

    // world
    //     .create_entities((
    //         &MeshComp::new(MeshID(hash64!("Sphere mesh"))),
    //         &PositionComp(Point3::new(10.0, 0.0, 0.0)),
    //         &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
    //         &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
    //         &BlinnPhongComp {
    //             ambient: vector![0.1, 0.1, 0.1],
    //             diffuse: vector![0.4, 0.4, 0.4],
    //             specular: vector![0.3, 0.3, 0.3],
    //             shininess: 6.0,
    //             alpha: 1.0,
    //         },
    //     ))
    //     .unwrap();

    // world
    //     .create_entities((
    //         &MeshComp::new(MeshID(hash64!("Pentagon mesh"))),
    //         &PositionComp(Point3::new(0.0, 10.0, 0.0)),
    //         &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
    //         &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
    //         &FixedColorComp(vector![1.0, 1.0, 1.0, 1.0]),
    //     ))
    //     .unwrap();

    // world
    //     .create_entities((
    //         &MeshComp::new(MeshID(hash64!("Cylinder mesh"))),
    //         &PositionComp(Point3::new(0.0, 0.0, 10.0)),
    //         &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
    //         &AngularVelocityComp(AngularVelocity::new(
    //             UnitVector3::new_normalize(vector![0.5, 0.2, 0.1]),
    //             Degrees(0.0),
    //         )),
    //         &BlinnPhongComp {
    //             ambient: vector![0.1, 0.1, 0.1],
    //             diffuse: vector![0.4, 0.4, 0.4],
    //             specular: vector![0.3, 0.3, 0.3],
    //             shininess: 6.0,
    //             alpha: 1.0,
    //         },
    //     ))
    //     .unwrap();

    // world
    //     .create_entities((
    //         &[
    //             PositionComp(Point3::new(8.0, 10.0, -10.0)),
    //             PositionComp(Point3::new(-5.0, 4.0, 8.0)),
    //         ],
    //         &[
    //             RadianceComp(vector![1.0, 1.0, 0.5] * 130.0),
    //             RadianceComp(vector![1.0, 0.7, 0.7] * 40.0),
    //         ],
    //         &[Omnidirectional, Omnidirectional],
    //     ))
    //     .unwrap();

    // world
    //     .create_entities((
    //         &DirectionComp(LightDirection::new_normalize(vector![0.6, -1.0, 1.0])),
    //         &RadianceComp(vector![1.0, 1.0, 1.0] * 0.3),
    //     ))
    //     .unwrap();
    world
        .create_entities((
            &[
                DirectionComp(LightDirection::new_normalize(vector![0.3, -1.0, 0.5])),
                DirectionComp(LightDirection::new_normalize(vector![-0.4, -0.6, -0.3])),
            ],
            &[
                RadianceComp(vector![1.0, 1.0, 1.0] * 0.3),
                RadianceComp(vector![1.0, 1.0, 1.0] * 0.1),
            ],
        ))
        .unwrap();

    Ok(world)
}
