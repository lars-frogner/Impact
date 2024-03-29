//! Running an event loop.

use super::{
    geometry::{Degrees, UpperExclusiveBounds},
    rendering::{CoreRenderingSystem, RenderingSystem},
};
use crate::{
    control::{
        MotionControlComp, OrientationControlComp, RollFreeCameraOrientationController,
        SemiDirectionalMotionController,
    },
    game_loop::{GameLoop, GameLoopConfig},
    geometry::{FrontFaceSide, VoxelTreeLODController, VoxelType},
    num::Float,
    physics::{
        fph, Acceleration, AngularVelocity, CircularTrajectoryComp,
        ConstantAccelerationTrajectoryComp, ConstantRotationComp, DetailedDragComp,
        HarmonicOscillatorTrajectoryComp, LogsKineticEnergy, LogsMomentum, OrbitalTrajectoryComp,
        Orientation, PhysicsSimulator, Position, ReferenceFrameComp, SimulatorConfig, Spring,
        SpringComp, UniformGravityComp, UniformMedium, UniformRigidBodyComp, VelocityComp,
    },
    rendering::{fre, Assets, ColorSpace, TextureAddressingConfig, TextureConfig},
    scene::{
        AngularExtentComp, BoxMeshComp, ConeMeshComp, CylinderMeshComp, DiffuseColorComp,
        DiffuseTextureComp, DirectionComp, EmissionExtentComp, EmissiveColorComp, LightDirection,
        MicrofacetDiffuseReflectionComp, MicrofacetSpecularReflectionComp, NormalMapComp,
        OmnidirectionalComp, ParallaxMapComp, ParentComp, PerspectiveCameraComp,
        PlanarTextureProjectionComp, RadianceComp, RectangleMeshComp, RoughnessComp,
        RoughnessTextureComp, Scene, SceneConfig, SceneGraphGroupComp, SkyboxComp,
        SpecularColorComp, SphereMeshComp, UncullableComp, VoxelBoxComp, VoxelSphereComp,
        VoxelTypeComp,
    },
    window::InputHandler,
    window::{KeyActionMap, Window},
    world::World,
};
use anyhow::Result;
use nalgebra::{point, vector, Point3, Vector3};
use rand::{rngs::ThreadRng, Rng, SeedableRng};
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

    let simulator = PhysicsSimulator::new(SimulatorConfig::default(), UniformMedium::vacuum())?;

    let motion_controller = SemiDirectionalMotionController::new(8.0, true);
    let orientation_controller =
        RollFreeCameraOrientationController::new(Degrees(f64::from(vertical_field_of_view.0)), 1.0);

    let scene = Scene::new(SceneConfig {
        initial_min_angular_voxel_extent_for_lod:
            VoxelTreeLODController::compute_min_angular_voxel_extent(
                window.dimensions().1,
                vertical_field_of_view,
                10.0,
            ),
        ..SceneConfig::default()
    });

    let world = World::new(
        window,
        scene,
        renderer,
        simulator,
        Some(Box::new(motion_controller)),
        Some(Box::new(orientation_controller)),
    );

    world
        .create_entity((
            // &CylinderMeshComp::new(1.8, 0.25, 30),
            // &SphereMeshComp::new(15),
            // &UniformRigidBodyComp { mass_density: 1e3 },
            &PerspectiveCameraComp::new(
                vertical_field_of_view,
                UpperExclusiveBounds::new(0.01, 500.0),
            ),
            &ReferenceFrameComp::unscaled(
                Point3::new(0.0, 7.0, -10.0),
                Orientation::from_axis_angle(&Vector3::y_axis(), PI),
            ),
            &VelocityComp::stationary(),
            &MotionControlComp::new(),
            &OrientationControlComp::new(),
            // &UniformGravityComp::downward(9.81),
        ))
        .unwrap();

    // world
    //     .create_entity((
    //         &BoxMeshComp::SKYBOX,
    //         &ReferenceFrameComp::default(),
    //         &SkyboxComp(skybox_texture_id),
    //         &UncullableComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &world.load_mesh_from_obj_file("assets/Dragon_1.obj")?,
    //         &ReferenceFrameComp::new(
    //             Point3::new(0.0, 1.5, 11.0),
    //             Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
    //             0.06,
    //         ),
    //         &DiffuseColorComp(vector![0.1, 0.2, 0.6]),
    //         &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 50.0),
    //         &RoughnessComp(0.4),
    //         &MicrofacetDiffuseReflectionComp,
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &CylinderMeshComp::new(10.0, 0.6, 100),
    //         &ReferenceFrameComp::unoriented(Point3::new(7.0, 0.5, 5.0)),
    //         &SpecularColorComp::IRON,
    //         &RoughnessComp(0.5),
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &world.load_mesh_from_obj_file("assets/abstract_object.obj")?,
    //         &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(7.0, 7.7, 5.0), 0.02),
    //         &ConstantRotationComp::new(
    //             0.0,
    //             Orientation::from_axis_angle(&Vector3::y_axis(), 0.0),
    //             AngularVelocity::new(Vector3::y_axis(), Degrees(50.0)),
    //         ),
    //         &SpecularColorComp::COPPER,
    //         &RoughnessComp(0.35),
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &world.load_mesh_from_obj_file("assets/abstract_pyramid.obj")?,
    //         &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(-1.0, 9.0, 9.0), 0.035),
    //         &ConstantRotationComp::new(
    //             0.0,
    //             Orientation::from_axis_angle(&Vector3::x_axis(), 0.4),
    //             AngularVelocity::new(Vector3::y_axis(), Degrees(-60.0)),
    //         ),
    //         &DiffuseColorComp(vector![0.7, 0.3, 0.2]),
    //         &RoughnessComp(0.95),
    //         &MicrofacetDiffuseReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &BoxMeshComp::UNIT_CUBE,
    //         &ReferenceFrameComp::unoriented_scaled(Point3::new(-9.0, -1.0, 5.0), 2.0),
    //         &DiffuseColorComp(vector![0.1, 0.7, 0.3]),
    //         &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 0.0),
    //         &RoughnessComp(0.55),
    //         &MicrofacetDiffuseReflectionComp,
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &SphereMeshComp::new(100),
    //         &ReferenceFrameComp::unoriented_scaled(Point3::new(-9.0, 2.0, 5.0), 4.0),
    //         &DiffuseColorComp(vector![0.3, 0.2, 0.7]),
    //         &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 0.5),
    //         &RoughnessComp(0.7),
    //         &MicrofacetDiffuseReflectionComp,
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &world.load_mesh_from_obj_file("assets/abstract_cube.obj")?,
    //         &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(-9.0, 5.8, 5.0), 0.016),
    //         &ConstantRotationComp::new(
    //             0.0,
    //             Orientation::from_axis_angle(&Vector3::y_axis(), 0.7),
    //             AngularVelocity::new(Vector3::x_axis(), Degrees(30.0)),
    //         ),
    //         &SpecularColorComp::GOLD,
    //         &RoughnessComp(0.4),
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &RectangleMeshComp::UNIT_SQUARE,
    //         &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
    //         &ReferenceFrameComp::new(
    //             Point3::new(0.0, -2.0, 0.0),
    //             Orientation::from_axis_angle(&Vector3::z_axis(), 0.0),
    //             50.0,
    //         ),
    //         &DiffuseTextureComp(wood_floor_color_texture_id),
    //         &SpecularColorComp::in_range_of(SpecularColorComp::LIVING_TISSUE, 100.0),
    //         &RoughnessTextureComp::unscaled(wood_floor_roughness_texture_id),
    //         &NormalMapComp(wood_floor_normal_texture_id),
    //         &MicrofacetDiffuseReflectionComp,
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &RectangleMeshComp::UNIT_SQUARE,
    //         &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
    //         &ReferenceFrameComp::new(
    //             Point3::new(25.0, 5.0, 0.0),
    //             Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
    //                 * Orientation::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
    //             50.0,
    //         ),
    //         &DiffuseTextureComp(bricks_color_texture_id),
    //         &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 100.0),
    //         &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
    //         &ParallaxMapComp::new(
    //             bricks_height_texture_id,
    //             0.02,
    //             vector![1.0 / 25.0, 1.0 / 25.0],
    //         ),
    //         &MicrofacetDiffuseReflectionComp,
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &RectangleMeshComp::UNIT_SQUARE,
    //         &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
    //         &ReferenceFrameComp::new(
    //             Point3::new(-25.0, 5.0, 0.0),
    //             Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
    //                 * Orientation::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
    //             50.0,
    //         ),
    //         &DiffuseTextureComp(bricks_color_texture_id),
    //         &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 100.0),
    //         &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
    //         &ParallaxMapComp::new(
    //             bricks_height_texture_id,
    //             0.02,
    //             vector![1.0 / 25.0, 1.0 / 25.0],
    //         ),
    //         &MicrofacetDiffuseReflectionComp,
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    // world
    //     .create_entity((
    //         &RectangleMeshComp::UNIT_SQUARE,
    //         &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
    //         &ReferenceFrameComp::new(
    //             Point3::new(0.0, 5.0, 25.0),
    //             Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
    //             50.0,
    //         ),
    //         &DiffuseTextureComp(bricks_color_texture_id),
    //         &SpecularColorComp::in_range_of(SpecularColorComp::STONE, 100.0),
    //         &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
    //         &ParallaxMapComp::new(
    //             bricks_height_texture_id,
    //             0.02,
    //             vector![1.0 / 25.0, 1.0 / 25.0],
    //         ),
    //         &MicrofacetDiffuseReflectionComp,
    //         &MicrofacetSpecularReflectionComp,
    //     ))
    //     .unwrap();

    world
        .create_entity((
            &SphereMeshComp::new(25),
            &ReferenceFrameComp::unoriented_scaled(Point3::new(0.0, 15.0, 2.0), 0.7),
            &RadianceComp(vector![1.0, 1.0, 1.0] * 150.0),
            &DiffuseColorComp(Vector3::zeros()),
            &EmissiveColorComp(vector![1.0, 1.0, 1.0]),
            &OmnidirectionalComp,
            &EmissionExtentComp(0.7),
        ))
        .unwrap();

    world
        .create_entity((
            &DirectionComp(LightDirection::new_normalize(vector![0.6, -0.3, 1.0])),
            &RadianceComp(vector![1.0, 1.0, 1.0] * 0.2),
            &AngularExtentComp(Degrees(2.0)),
        ))
        .unwrap();

    world
        .create_entity(&RadianceComp::for_uniform_irradiance(
            &(vector![1.0, 1.0, 1.0] * 0.05),
        ))
        .unwrap();

    world
        .create_entity((
            &VoxelSphereComp::new(500, 4),
            &VoxelTypeComp::new(VoxelType::Default),
            &ReferenceFrameComp::unoriented(point![-100.0, -100.0, -4.0]),
        ))
        .unwrap();

    // create_harmonic_oscillation_experiment(&world, Point3::new(0.0, 10.0, 2.0), 1.0, 10.0, 3.0);
    // create_free_rotation_experiment(&world, Point3::new(0.0, 7.0, 2.0), 5.0, 1e-3);
    // create_drag_drop_experiment(&world, Point3::new(0.0, 20.0, 4.0));

    Ok(world)
}

fn create_harmonic_oscillation_experiment(
    world: &World,
    position: Position,
    mass: fph,
    spring_constant: fph,
    amplitude: fph,
) {
    let angular_frequency = fph::sqrt(spring_constant / mass);
    let period = fph::TWO_PI / angular_frequency;

    let attachment_position = position;
    let mass_position = attachment_position + vector![0.0, -2.0 * amplitude - 0.5, 0.0];

    let reference_position = attachment_position + vector![-2.0, -amplitude - 0.5, 0.0];

    let attachment_point_entity = world
        .create_entity((
            &SphereMeshComp::new(15),
            &ReferenceFrameComp::unoriented_scaled(attachment_position, 0.2),
            &DiffuseColorComp(vector![0.8, 0.1, 0.1]),
        ))
        .unwrap();

    let cube_body_entity = world
        .create_entity((
            &BoxMeshComp::UNIT_CUBE,
            &UniformRigidBodyComp { mass_density: mass },
            &ReferenceFrameComp::for_unoriented_rigid_body(mass_position),
            &VelocityComp::stationary(),
            &DiffuseColorComp(vector![0.1, 0.1, 0.7]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 80.0),
            &LogsKineticEnergy,
            &LogsMomentum,
        ))
        .unwrap();

    world
        .create_entity((
            &ReferenceFrameComp::default(),
            &SpringComp::new(
                attachment_point_entity,
                cube_body_entity,
                Position::origin(),
                Position::origin(),
                Spring::standard(spring_constant, 0.0, amplitude + 0.5),
            ),
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::UNIT_CUBE,
            &ReferenceFrameComp::for_driven_trajectory(Orientation::identity()),
            &VelocityComp::stationary(),
            &HarmonicOscillatorTrajectoryComp::new(
                0.25 * period,
                reference_position,
                Vector3::y_axis(),
                amplitude,
                period,
            ),
            &DiffuseColorComp(vector![0.1, 0.7, 0.1]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 80.0),
        ))
        .unwrap();
}

fn create_free_rotation_experiment(
    world: &World,
    position: Position,
    angular_speed: fph,
    angular_velocity_perturbation_fraction: fph,
) {
    let major_axis_body_position = position + vector![5.0, 0.0, 0.0];
    let intermediate_axis_body_position = position;
    let minor_axis_body_position = position - vector![5.0, 0.0, 0.0];

    let angular_velocity_perturbation = angular_speed * angular_velocity_perturbation_fraction;

    world
        .create_entity((
            &BoxMeshComp::new(3.0, 2.0, 1.0, FrontFaceSide::Outside),
            &UniformRigidBodyComp {
                mass_density: 1.0 / 6.0,
            },
            &ReferenceFrameComp::for_unoriented_rigid_body(major_axis_body_position),
            &VelocityComp::angular(AngularVelocity::from_vector(vector![
                angular_velocity_perturbation,
                angular_velocity_perturbation,
                angular_speed
            ])),
            &DiffuseColorComp(vector![0.1, 0.1, 0.7]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 80.0),
            &LogsKineticEnergy,
            &LogsMomentum,
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::new(3.0, 2.0, 1.0, FrontFaceSide::Outside),
            &UniformRigidBodyComp {
                mass_density: 1.0 / 6.0,
            },
            &ReferenceFrameComp::for_unoriented_rigid_body(intermediate_axis_body_position),
            &VelocityComp::angular(AngularVelocity::from_vector(vector![
                angular_velocity_perturbation,
                angular_speed,
                angular_velocity_perturbation
            ])),
            &DiffuseColorComp(vector![0.1, 0.1, 0.7]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 80.0),
            &LogsKineticEnergy,
            &LogsMomentum,
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::new(3.0, 2.0, 1.0, FrontFaceSide::Outside),
            &UniformRigidBodyComp {
                mass_density: 1.0 / 6.0,
            },
            &ReferenceFrameComp::for_unoriented_rigid_body(minor_axis_body_position),
            &VelocityComp::angular(AngularVelocity::from_vector(vector![
                angular_speed,
                angular_velocity_perturbation,
                angular_velocity_perturbation
            ])),
            &DiffuseColorComp(vector![0.1, 0.1, 0.7]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 80.0),
            &LogsKineticEnergy,
            &LogsMomentum,
        ))
        .unwrap();
}

fn create_drag_drop_experiment(world: &World, position: Position) {
    world
        .simulator()
        .write()
        .unwrap()
        .set_medium(UniformMedium::moving_air(vector![0.0, 3.0, 0.0]));

    world
        .create_entity((
            // &SphereMeshComp::new(100),
            &ConeMeshComp::new(2.0, 1.0, 100),
            // &BoxMeshComp::new(3.0, 0.4, 1.0, FrontFaceSide::Outside),
            &UniformRigidBodyComp { mass_density: 10.0 },
            &ReferenceFrameComp::for_rigid_body(
                position,
                Orientation::from_axis_angle(&Vector3::z_axis(), 3.0),
            ),
            &VelocityComp::angular(AngularVelocity::zero()),
            &DiffuseColorComp(vector![0.1, 0.1, 0.7]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 80.0),
            &UniformGravityComp::earth(),
            &DetailedDragComp::new(1.0),
            &LogsKineticEnergy,
            &LogsMomentum,
        ))
        .unwrap();

    world
        .create_entity((
            &ConeMeshComp::new(2.0, 1.0, 100),
            &UniformRigidBodyComp { mass_density: 10.0 },
            &ReferenceFrameComp::for_rigid_body(
                position + vector![-5.0, 0.0, 0.0],
                Orientation::from_axis_angle(&Vector3::z_axis(), 3.0),
            ),
            &VelocityComp::angular(AngularVelocity::zero()),
            &DiffuseColorComp(vector![0.7, 0.1, 0.1]),
            &SpecularColorComp::in_range_of(SpecularColorComp::PLASTIC, 80.0),
            &UniformGravityComp::earth(),
        ))
        .unwrap();
}
