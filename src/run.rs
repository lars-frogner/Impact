//! Running an event loop.

use super::{
    geometry::{Degrees, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, ImageTexture, RenderingSystem},
};
use crate::{
    control::{Controllable, RollFreeCameraOrientationController, SemiDirectionalMotionController},
    game_loop::{GameLoop, GameLoopConfig},
    geometry::TriangleMesh,
    physics::{
        AngularVelocity, AngularVelocityComp, Orientation, OrientationComp, PhysicsSimulator,
        PositionComp, SimulatorConfig, VelocityComp,
    },
    rendering::{Assets, TextureID},
    scene::{
        AngularExtentComp, DiffuseColorComp, DiffuseTextureComp, DirectionComp, EmissionExtentComp,
        FixedColorComp, LightDirection, MeshComp, MeshID, MeshRepository,
        MicrofacetDiffuseReflection, MicrofacetSpecularReflection, Omnidirectional,
        PerspectiveCameraComp, RadianceComp, RoughnessComp, ScalingComp, Scene, SpecularColorComp,
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

    assets.image_textures.insert(
        TextureID(hash32!("Wood texture")),
        ImageTexture::from_path(&core_system, "assets/Wood049_4K-JPG/Wood049_4K_Color.jpg")?,
    );
    assets.image_textures.insert(
        TextureID(hash32!("Plaster texture")),
        ImageTexture::from_path(
            &core_system,
            "assets/PaintedPlaster017_4K-JPG/PaintedPlaster017_4K_Color.jpg",
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
            MeshID(hash64!("Pole mesh")),
            TriangleMesh::create_cylinder(10.0, 1.0, 100),
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

    let vertical_field_of_view = Degrees(70.0);
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
                        &PositionComp(Point3::new(6.3, 6.5 - 1.7, 3.7)),
                        &ScalingComp(1.0),
                        &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
                        &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
                        &DiffuseColorComp(vector![0.2, 0.3, 0.7]),
                        &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 50.0),
                        &RoughnessComp(0.4),
                        &MicrofacetDiffuseReflection,
                        &MicrofacetSpecularReflection,
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
                        &PositionComp(Point3::new(-1.0, 1.5, 8.0)),
                        &ScalingComp(0.17),
                        &OrientationComp(Orientation::from_axis_angle(
                            &Vector3::x_axis(),
                            -PI / 2.0 + 0.4,
                        )),
                        &AngularVelocityComp(AngularVelocity::new(
                            Vector3::y_axis(),
                            Degrees(-2.3),
                        )),
                        &DiffuseColorComp(vector![0.8, 0.4, 0.3]),
                        &RoughnessComp(0.9),
                        &MicrofacetDiffuseReflection,
                    ))
                    .unwrap(),
            )
            .unwrap();
    }

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Sphere mesh"))),
            &PositionComp(Point3::new(-6.0, -1.0, 4.0)),
            &ScalingComp(2.0),
            &SpecularColorComp::GOLD,
            &RoughnessComp(0.4),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Box mesh"))),
            &PositionComp(Point3::new(-6.0, 1.0, 4.0)),
            &ScalingComp(2.0),
            &OrientationComp(Orientation::identity()),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(1.3))),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 0.0),
            &DiffuseColorComp(vector![0.2, 0.8, 0.4]),
            &RoughnessComp(0.55),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Pole mesh"))),
            &PositionComp(Point3::new(6.0, 0.5, 4.0)),
            &ScalingComp(1.0),
            &SpecularColorComp::IRON,
            &RoughnessComp(0.5),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Plane mesh"))),
            &PositionComp(Point3::new(0.0, -2.0, 0.0)),
            &ScalingComp(50.0),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::z_axis(), 0.0)),
            &DiffuseTextureComp(TextureID(hash32!("Wood texture"))),
            &SpecularColorComp::in_range_of(SpecularColorComp::LIVING_TISSUE, 100.0),
            &RoughnessComp(0.85),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Plane mesh"))),
            &PositionComp(Point3::new(25.0, 0.0, 0.0)),
            &ScalingComp(50.0),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::z_axis(), PI / 2.0)),
            &DiffuseTextureComp(TextureID(hash32!("Plaster texture"))),
            &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 80.0),
            &RoughnessComp(0.75),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Plane mesh"))),
            &PositionComp(Point3::new(-25.0, 0.0, 0.0)),
            &ScalingComp(50.0),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::z_axis(), -PI / 2.0)),
            &DiffuseTextureComp(TextureID(hash32!("Plaster texture"))),
            &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 80.0),
            &RoughnessComp(0.75),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Plane mesh"))),
            &PositionComp(Point3::new(0.0, 0.0, 25.0)),
            &ScalingComp(50.0),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)),
            &DiffuseTextureComp(TextureID(hash32!("Plaster texture"))),
            &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 80.0),
            &RoughnessComp(0.75),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entities((
            &PerspectiveCameraComp::new(
                vertical_field_of_view,
                UpperExclusiveBounds::new(0.1, 100.0),
            ),
            &PositionComp(Point3::new(0.0, 2.0, -8.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), PI)),
            &VelocityComp(Vector3::zeros()),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
            &Controllable,
        ))
        .unwrap();

    // world
    //     .create_entities((
    //         &MeshComp::new(MeshID(hash64!("Sphere mesh"))),
    //         &ScalingComp(0.2),
    //         &PositionComp(Point3::new(0.0, 1.0, -2.0)),
    //         &RadianceComp(vector![1.0, 1.0, 1.0] * 15.0),
    //         &FixedColorComp(vector![1.0, 1.0, 1.0]),
    //         &Omnidirectional,
    //     ))
    //     .unwrap();

    world
        .create_entities((
            &MeshComp::new(MeshID(hash64!("Sphere mesh"))),
            &ScalingComp(0.7),
            &PositionComp(Point3::new(0.0, 9.0, 2.0)),
            &RadianceComp(vector![1.0, 1.0, 1.0] * 60.0),
            &FixedColorComp(vector![1.0, 1.0, 1.0]),
            &Omnidirectional,
            &EmissionExtentComp(0.7),
        ))
        .unwrap();

    world
        .create_entities((
            &[
                DirectionComp(LightDirection::new_normalize(vector![-0.3, -0.7, 1.0])),
                DirectionComp(LightDirection::new_normalize(vector![0.6, -0.3, 1.0])),
            ],
            &[
                RadianceComp(vector![1.0, 1.0, 1.0] * 0.25),
                RadianceComp(vector![1.0, 1.0, 1.0] * 0.20),
            ],
            &[
                AngularExtentComp(Degrees(2.0)),
                AngularExtentComp(Degrees(2.0)),
            ],
        ))
        .unwrap();

    Ok(world)
}
