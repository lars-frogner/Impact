//! Running an event loop.

use super::{
    geometry::{Degrees, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, RenderingSystem},
};
use crate::{
    control::{Controllable, RollFreeCameraOrientationController, SemiDirectionalMotionController},
    game_loop::{GameLoop, GameLoopConfig},
    physics::{
        AngularVelocity, AngularVelocityComp, Orientation, PhysicsSimulator, Position,
        SimulatorConfig, SpatialConfigurationComp, Spring, SpringComp, UniformGravityComp,
        UniformRigidBodyComp, VelocityComp,
    },
    rendering::{Assets, ColorSpace, TextureAddressingConfig, TextureConfig},
    scene::{
        AngularExtentComp, BoxMeshComp, CylinderMeshComp, DiffuseColorComp, DiffuseTextureComp,
        DirectionComp, EmissionExtentComp, EmissiveColorComp, LightDirection,
        MicrofacetDiffuseReflection, MicrofacetSpecularReflection, NormalMapComp, Omnidirectional,
        ParallaxMapComp, ParentComp, PerspectiveCameraComp, PlanarTextureProjectionComp,
        RadianceComp, RectangleMeshComp, RoughnessComp, RoughnessTextureComp, ScalingComp,
        SceneGraphGroup, SkyboxComp, SpecularColorComp, SphereMeshComp, Uncullable,
        UniformIrradianceComp,
    },
    window::InputHandler,
    window::{KeyActionMap, Window},
    world::World,
};
use anyhow::Result;
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
        .run_game_loop(GameLoop::new(world, input_handler, GameLoopConfig::default()).unwrap())
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

    let mut assets = Assets::new(&core_system);

    let skybox_texture_id = assets.load_cubemap_texture_from_paths(
        &core_system,
        "assets/skybox/right.jpg",
        "assets/skybox/left.jpg",
        "assets/skybox/top.jpg",
        "assets/skybox/bottom.jpg",
        "assets/skybox/front.jpg",
        "assets/skybox/back.jpg",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            ..Default::default()
        },
    )?;

    let bricks_color_texture_id = assets.load_texture_from_path(
        &core_system,
        "assets/Bricks059_4K-JPG/Bricks059_4K_Color.jpg",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let bricks_roughness_texture_id = assets.load_texture_from_path(
        &core_system,
        "assets/Bricks059_4K-JPG/Bricks059_4K_Roughness.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let bricks_height_texture_id = assets.load_texture_from_path(
        &core_system,
        "assets/Bricks059_4K-JPG/Bricks059_4K_Displacement.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let wood_floor_color_texture_id = assets.load_texture_from_path(
        &core_system,
        "assets/WoodFloor041_4K-JPG/WoodFloor041_4K_Color.jpg",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let wood_floor_roughness_texture_id = assets.load_texture_from_path(
        &core_system,
        "assets/WoodFloor041_4K-JPG/WoodFloor041_4K_Roughness.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let wood_floor_normal_texture_id = assets.load_texture_from_path(
        &core_system,
        "assets/WoodFloor041_4K-JPG/WoodFloor041_4K_NormalDX.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let vertical_field_of_view = Degrees(70.0);
    let renderer = RenderingSystem::new(core_system, assets).await?;

    let simulator = PhysicsSimulator::new(SimulatorConfig::default());

    let motion_controller = SemiDirectionalMotionController::new(8.0, true);
    let orientation_controller =
        RollFreeCameraOrientationController::new(Degrees(f64::from(vertical_field_of_view.0)), 1.0);

    let world = World::new(
        window,
        renderer,
        simulator,
        Some(Box::new(motion_controller)),
        Some(Box::new(orientation_controller)),
    );

    world
        .create_entity((
            &PerspectiveCameraComp::new(
                vertical_field_of_view,
                UpperExclusiveBounds::new(0.1, 100.0),
            ),
            &SpatialConfigurationComp::new(
                Point3::new(0.0, 7.0, -10.0),
                Orientation::from_axis_angle(&Vector3::y_axis(), PI),
            ),
            &VelocityComp(Vector3::zeros()),
            &AngularVelocityComp(AngularVelocity::zero()),
            &Controllable,
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::SKYBOX,
            &SpatialConfigurationComp::default(),
            &SkyboxComp(skybox_texture_id),
            &Uncullable,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/Dragon_1.obj")?,
            &ScalingComp(0.06),
            &SpatialConfigurationComp::new(
                Point3::new(0.0, 1.5, 11.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
            ),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
            &DiffuseColorComp(vector![0.1, 0.2, 0.6]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 50.0),
            &RoughnessComp(0.4),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &CylinderMeshComp::new(10.0, 0.6, 100),
            &ScalingComp(1.0),
            &SpatialConfigurationComp::unoriented(Point3::new(7.0, 0.5, 5.0)),
            &SpecularColorComp::IRON,
            &RoughnessComp(0.5),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_object.obj")?,
            &ScalingComp(0.02),
            &SpatialConfigurationComp::new(
                Point3::new(7.0, 7.7, 5.0),
                Orientation::from_axis_angle(&Vector3::y_axis(), 0.0),
            ),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(50.0))),
            &SpecularColorComp::COPPER,
            &RoughnessComp(0.35),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_pyramid.obj")?,
            &ScalingComp(0.035),
            &SpatialConfigurationComp::new(
                Point3::new(-1.0, 9.0, 9.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), 0.4),
            ),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(-60.0))),
            &DiffuseColorComp(vector![0.7, 0.3, 0.2]),
            &RoughnessComp(0.95),
            &MicrofacetDiffuseReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::UNIT_CUBE,
            &ScalingComp(2.0),
            &SpatialConfigurationComp::unoriented(Point3::new(-9.0, -1.0, 5.0)),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(0.0))),
            &DiffuseColorComp(vector![0.1, 0.7, 0.3]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 0.0),
            &RoughnessComp(0.55),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &SphereMeshComp::new(100),
            &ScalingComp(4.0),
            &SpatialConfigurationComp::unoriented(Point3::new(-9.0, 2.0, 5.0)),
            &DiffuseColorComp(vector![0.3, 0.2, 0.7]),
            &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 0.5),
            &RoughnessComp(0.7),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_cube.obj")?,
            &ScalingComp(0.016),
            &SpatialConfigurationComp::new(
                Point3::new(-9.0, 5.8, 5.0),
                Orientation::from_axis_angle(&Vector3::y_axis(), 0.7),
            ),
            &AngularVelocityComp(AngularVelocity::new(Vector3::x_axis(), Degrees(30.0))),
            &SpecularColorComp::GOLD,
            &RoughnessComp(0.4),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ScalingComp(50.0),
            &SpatialConfigurationComp::new(
                Point3::new(0.0, -2.0, 0.0),
                Orientation::from_axis_angle(&Vector3::z_axis(), 0.0),
            ),
            &DiffuseTextureComp(wood_floor_color_texture_id),
            &SpecularColorComp::in_range_of(SpecularColorComp::LIVING_TISSUE, 100.0),
            &RoughnessTextureComp::unscaled(wood_floor_roughness_texture_id),
            &NormalMapComp(wood_floor_normal_texture_id),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ScalingComp(50.0),
            &SpatialConfigurationComp::new(
                Point3::new(25.0, 5.0, 0.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * Orientation::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
            ),
            &DiffuseTextureComp(bricks_color_texture_id),
            &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 100.0),
            &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
            &ParallaxMapComp::new(
                bricks_height_texture_id,
                0.02,
                vector![1.0 / 25.0, 1.0 / 25.0],
            ),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ScalingComp(50.0),
            &SpatialConfigurationComp::new(
                Point3::new(-25.0, 5.0, 0.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * Orientation::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
            ),
            &DiffuseTextureComp(bricks_color_texture_id),
            &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 100.0),
            &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
            &ParallaxMapComp::new(
                bricks_height_texture_id,
                0.02,
                vector![1.0 / 25.0, 1.0 / 25.0],
            ),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ScalingComp(50.0),
            &SpatialConfigurationComp::new(
                Point3::new(0.0, 5.0, 25.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
            ),
            &DiffuseTextureComp(bricks_color_texture_id),
            &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 100.0),
            &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
            &ParallaxMapComp::new(
                bricks_height_texture_id,
                0.02,
                vector![1.0 / 25.0, 1.0 / 25.0],
            ),
            &MicrofacetDiffuseReflection,
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &SphereMeshComp::new(25),
            &ScalingComp(0.7),
            &SpatialConfigurationComp::unoriented(Point3::new(0.0, 15.0, 2.0)),
            &RadianceComp(vector![1.0, 1.0, 1.0] * 30.0),
            &DiffuseColorComp(Vector3::zeros()),
            &EmissiveColorComp(vector![1.0, 1.0, 1.0]),
            &Omnidirectional,
            &EmissionExtentComp(0.7),
        ))
        .unwrap();

    world
        .create_entity((
            &DirectionComp(LightDirection::new_normalize(vector![0.6, -0.3, 1.0])),
            &RadianceComp(vector![1.0, 1.0, 1.0] * 0.05),
            &AngularExtentComp(Degrees(2.0)),
        ))
        .unwrap();

    let parent = world
        .create_entity((
            &SpatialConfigurationComp::unoriented(Point3::new(0.0, 15.0, 5.0)),
            &AngularVelocityComp(AngularVelocity::new(Vector3::y_axis(), Degrees(35.0))),
            &SceneGraphGroup,
        ))
        .unwrap();

    world
        .create_entity((
            &SphereMeshComp::new(25),
            &ScalingComp(3.0),
            &SpatialConfigurationComp::unoriented(Point3::new(10.0, 0.0, 0.0)),
            &RadianceComp(vector![1.0, 1.0, 1.0] * 80.0),
            &DiffuseColorComp(Vector3::zeros()),
            &EmissiveColorComp(vector![1.0, 1.0, 1.0]),
            &Omnidirectional,
            &EmissionExtentComp(3.0),
            &ParentComp::new(parent),
        ))
        .unwrap();

    world
        .create_entity(&UniformIrradianceComp(vector![1.0, 1.0, 1.0] * 0.03))
        .unwrap();

    Ok(world)
}
