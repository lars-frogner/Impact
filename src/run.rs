//! Running an event loop.

use super::{
    geometry::{Degrees, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, RenderingSystem},
};
use crate::{
    control::{Controllable, RollFreeCameraOrientationController, SemiDirectionalMotionController},
    game_loop::{GameLoop, GameLoopConfig},
    physics::{
        AngularVelocity, DrivenAngularVelocityComp, Orientation, OrientationComp, PhysicsSimulator,
        PositionComp, SimulatorConfig, VelocityComp,
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
            &PositionComp(Point3::new(0.0, 7.0, -10.0)),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), PI)),
            &VelocityComp(Vector3::zeros()),
            &DrivenAngularVelocityComp::new_about_model_origin(AngularVelocity::new(
                Vector3::y_axis(),
                Degrees(0.0),
            )),
            &Controllable,
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::SKYBOX,
            &PositionComp(Point3::origin()),
            &SkyboxComp(skybox_texture_id),
            &Uncullable,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/Dragon_1.obj")?,
            &PositionComp(Point3::new(0.0, 1.5, 11.0)),
            &ScalingComp(0.06),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)),
            &DrivenAngularVelocityComp::new_about_model_origin(AngularVelocity::new(
                Vector3::y_axis(),
                Degrees(0.0),
            )),
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
            &PositionComp(Point3::new(7.0, 0.5, 5.0)),
            &ScalingComp(1.0),
            &SpecularColorComp::IRON,
            &RoughnessComp(0.5),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_object.obj")?,
            &PositionComp(Point3::new(7.0, 7.7, 5.0)),
            &ScalingComp(0.02),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.0)),
            &DrivenAngularVelocityComp::new_about_model_origin(AngularVelocity::new(
                Vector3::y_axis(),
                Degrees(1.0),
            )),
            &SpecularColorComp::COPPER,
            &RoughnessComp(0.35),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_pyramid.obj")?,
            &PositionComp(Point3::new(-1.0, 9.0, 9.0)),
            &ScalingComp(0.035),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::x_axis(), 0.4)),
            &DrivenAngularVelocityComp::new_about_model_origin(AngularVelocity::new(
                Vector3::y_axis(),
                Degrees(-1.3),
            )),
            &DiffuseColorComp(vector![0.7, 0.3, 0.2]),
            &RoughnessComp(0.95),
            &MicrofacetDiffuseReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::UNIT_CUBE,
            &PositionComp(Point3::new(-9.0, -1.0, 5.0)),
            &ScalingComp(2.0),
            &OrientationComp(Orientation::identity()),
            &DrivenAngularVelocityComp::new_about_model_origin(AngularVelocity::new(
                Vector3::y_axis(),
                Degrees(0.0),
            )),
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
            &PositionComp(Point3::new(-9.0, 2.0, 5.0)),
            &ScalingComp(4.0),
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
            &PositionComp(Point3::new(-9.0, 5.8, 5.0)),
            &ScalingComp(0.016),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::y_axis(), 0.7)),
            &DrivenAngularVelocityComp::new_about_model_origin(AngularVelocity::new(
                Vector3::x_axis(),
                Degrees(0.7),
            )),
            &SpecularColorComp::GOLD,
            &RoughnessComp(0.4),
            &MicrofacetSpecularReflection,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &PositionComp(Point3::new(0.0, -2.0, 0.0)),
            &ScalingComp(50.0),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::z_axis(), 0.0)),
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
            &PositionComp(Point3::new(25.0, 5.0, 0.0)),
            &ScalingComp(50.0),
            &OrientationComp(
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
            &PositionComp(Point3::new(-25.0, 5.0, 0.0)),
            &ScalingComp(50.0),
            &OrientationComp(
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
            &PositionComp(Point3::new(0.0, 5.0, 25.0)),
            &ScalingComp(50.0),
            &OrientationComp(Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0)),
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
            &PositionComp(Point3::new(0.0, 9.0, 2.0)),
            &RadianceComp(vector![1.0, 1.0, 1.0] * 20.0),
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
            &PositionComp(Point3::new(0.0, 15.0, 5.0)),
            &OrientationComp(Orientation::identity()),
            &DrivenAngularVelocityComp::new_about_model_origin(AngularVelocity::new(
                Vector3::y_axis(),
                Degrees(1.5),
            )),
            &SceneGraphGroup,
        ))
        .unwrap();

    world
        .create_entity((
            &SphereMeshComp::new(25),
            &ScalingComp(3.0),
            &PositionComp(Point3::new(10.0, 0.0, 0.0)),
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
