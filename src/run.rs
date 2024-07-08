//! Running an event loop.

use super::{geometry::Degrees, gpu::rendering::RenderingSystem};
use crate::{
    assets::Assets,
    camera::components::PerspectiveCameraComp,
    control::{
        components::{MotionControlComp, OrientationControlComp},
        RollFreeCameraOrientationController, SemiDirectionalMotionController,
    },
    game_loop::{GameLoop, GameLoopConfig},
    gpu::{
        self,
        rendering::{fre, ColorSpace, RenderingConfig, TextureAddressingConfig, TextureConfig},
    },
    light::components::{
        AmbientEmissionComp, OmnidirectionalEmissionComp, UnidirectionalEmissionComp,
    },
    material::{
        self,
        components::{
            AlbedoComp, AlbedoTextureComp, EmissiveLuminanceComp, MicrofacetDiffuseReflectionComp,
            MicrofacetSpecularReflectionComp, NormalMapComp, ParallaxMapComp, RoughnessComp,
            RoughnessTextureComp, SkyboxComp, SpecularReflectanceComp,
        },
        MaterialLibrary,
    },
    mesh::{
        components::{
            BoxMeshComp, ConeMeshComp, CylinderMeshComp, RectangleMeshComp, SphereMeshComp,
        },
        texture_projection::components::PlanarTextureProjectionComp,
        FrontFaceSide, MeshRepository,
    },
    model::InstanceFeatureManager,
    num::Float,
    physics::{
        fph, Acceleration, AngularVelocity, CircularTrajectoryComp,
        ConstantAccelerationTrajectoryComp, ConstantRotationComp, DetailedDragComp,
        HarmonicOscillatorTrajectoryComp, LogsKineticEnergy, LogsMomentum, OrbitalTrajectoryComp,
        Orientation, PhysicsSimulator, Position, ReferenceFrameComp, SimulatorConfig, Spring,
        SpringComp, UniformGravityComp, UniformMedium, UniformRigidBodyComp, VelocityComp,
    },
    scene::{
        components::{ParentComp, SceneGraphGroupComp, UncullableComp},
        Scene,
    },
    util::bounds::UpperExclusiveBounds,
    voxel::{
        components::{VoxelSphereComp, VoxelTypeComp},
        VoxelConfig, VoxelManager, VoxelTreeLODController, VoxelType,
    },
    window::{GameHandler, InputHandler, KeyActionMap, Window},
    world::World,
};
use anyhow::Result;
use nalgebra::{point, vector, Point3, UnitVector3, Vector3};
use rand::{rngs::ThreadRng, Rng, SeedableRng};
use std::{f64::consts::PI, sync::Arc};

pub fn run() -> Result<()> {
    init_logging()?;
    let mut handler = GameHandler::new(init_game_loop);
    handler.run()
}

fn init_logging() -> Result<()> {
    env_logger::init();
    Ok(())
}

fn init_game_loop(window: Window) -> Result<GameLoop> {
    let world = init_world(window)?;
    let input_handler = InputHandler::new(KeyActionMap::default());
    GameLoop::new(world, input_handler, GameLoopConfig::default())
}

fn init_world(window: Window) -> Result<World> {
    let (graphics_device, rendering_surface) = gpu::initialize_for_rendering(&window)?;

    let vertical_field_of_view = Degrees(70.0);

    let mut assets = Assets::new_with_default_lookup_tables(&graphics_device)?;

    let skybox_texture_id = assets.load_cubemap_texture_from_paths(
        &graphics_device,
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
        &graphics_device,
        "assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Color.jpg",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let bricks_roughness_texture_id = assets.load_texture_from_path(
        &graphics_device,
        "assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Roughness.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let bricks_height_texture_id = assets.load_texture_from_path(
        &graphics_device,
        "assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Displacement.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let wood_floor_color_texture_id = assets.load_texture_from_path(
        &graphics_device,
        "assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_Color.jpg",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let wood_floor_roughness_texture_id = assets.load_texture_from_path(
        &graphics_device,
        "assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_Roughness.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let wood_floor_normal_texture_id = assets.load_texture_from_path(
        &graphics_device,
        "assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_NormalDX.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        },
    )?;

    let mut mesh_repository = MeshRepository::new();
    mesh_repository.create_default_meshes();

    let mut instance_feature_manager = InstanceFeatureManager::new();
    material::register_material_feature_types(&mut instance_feature_manager);

    let mut material_library = MaterialLibrary::new();

    let voxel_manager = VoxelManager::create(
        VoxelConfig {
            initial_min_angular_voxel_extent_for_lod:
                VoxelTreeLODController::compute_min_angular_voxel_extent(
                    window.dimensions().1,
                    vertical_field_of_view,
                    3.0,
                ),
            ..VoxelConfig::default()
        },
        &graphics_device,
        &assets,
        &mut mesh_repository,
        &mut material_library,
        &mut instance_feature_manager,
    );

    let renderer = RenderingSystem::new(
        RenderingConfig::default(),
        Arc::clone(&graphics_device),
        rendering_surface,
        &mut material_library,
    )?;

    let simulator = PhysicsSimulator::new(SimulatorConfig::default(), UniformMedium::vacuum())?;

    let motion_controller = SemiDirectionalMotionController::new(8.0, true);
    let orientation_controller =
        RollFreeCameraOrientationController::new(Degrees(f64::from(vertical_field_of_view.0)), 1.0);

    let scene = Scene::new(
        mesh_repository,
        material_library,
        instance_feature_manager,
        voxel_manager,
    );

    let world = World::new(
        Arc::new(window),
        graphics_device,
        renderer,
        assets,
        scene,
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

    world
        .create_entity((
            &BoxMeshComp::SKYBOX,
            &ReferenceFrameComp::default(),
            &SkyboxComp(skybox_texture_id),
            &UncullableComp,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/Dragon_1.obj")?,
            &ReferenceFrameComp::new(
                Point3::new(0.0, 1.5, 11.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
                0.06,
            ),
            &AlbedoComp(vector![0.1, 0.2, 0.6]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 50.0),
            &RoughnessComp(0.4),
            &MicrofacetDiffuseReflectionComp,
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &CylinderMeshComp::new(10.0, 0.6, 100),
            &ReferenceFrameComp::unoriented(Point3::new(7.0, 0.5, 5.0)),
            &SpecularReflectanceComp::IRON,
            &RoughnessComp(0.5),
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_object.obj")?,
            &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(7.0, 7.7, 5.0), 0.02),
            &ConstantRotationComp::new(
                0.0,
                Orientation::from_axis_angle(&Vector3::y_axis(), 0.0),
                AngularVelocity::new(Vector3::y_axis(), Degrees(50.0)),
            ),
            &SpecularReflectanceComp::COPPER,
            &RoughnessComp(0.35),
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_pyramid.obj")?,
            &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(-1.0, 9.0, 9.0), 0.035),
            &ConstantRotationComp::new(
                0.0,
                Orientation::from_axis_angle(&Vector3::x_axis(), 0.4),
                AngularVelocity::new(Vector3::y_axis(), Degrees(-60.0)),
            ),
            &AlbedoComp(vector![0.7, 0.3, 0.2]),
            &RoughnessComp(0.95),
            &MicrofacetDiffuseReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &BoxMeshComp::UNIT_CUBE,
            &ReferenceFrameComp::unoriented_scaled(Point3::new(-9.0, -1.0, 5.0), 2.0),
            &AlbedoComp(vector![0.1, 0.7, 0.3]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 0.0),
            &RoughnessComp(0.55),
            &MicrofacetDiffuseReflectionComp,
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &SphereMeshComp::new(100),
            &ReferenceFrameComp::unoriented_scaled(Point3::new(-9.0, 2.0, 5.0), 4.0),
            &AlbedoComp(vector![0.3, 0.2, 0.7]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::STONE, 0.5),
            &RoughnessComp(0.7),
            &MicrofacetDiffuseReflectionComp,
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &world.load_mesh_from_obj_file("assets/abstract_cube.obj")?,
            &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(-9.0, 5.8, 5.0), 0.016),
            &ConstantRotationComp::new(
                0.0,
                Orientation::from_axis_angle(&Vector3::y_axis(), 0.7),
                AngularVelocity::new(Vector3::x_axis(), Degrees(30.0)),
            ),
            &SpecularReflectanceComp::GOLD,
            &RoughnessComp(0.4),
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ReferenceFrameComp::new(
                Point3::new(0.0, -2.0, 0.0),
                Orientation::from_axis_angle(&Vector3::z_axis(), 0.0),
                50.0,
            ),
            &AlbedoTextureComp(wood_floor_color_texture_id),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::LIVING_TISSUE, 100.0),
            &RoughnessTextureComp::unscaled(wood_floor_roughness_texture_id),
            &NormalMapComp(wood_floor_normal_texture_id),
            &MicrofacetDiffuseReflectionComp,
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ReferenceFrameComp::new(
                Point3::new(25.0, 5.0, 0.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * Orientation::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
                50.0,
            ),
            &AlbedoTextureComp(bricks_color_texture_id),
            &SpecularReflectanceComp(vector![1.0, 1.0, 1.0] * 0.02),
            &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
            &ParallaxMapComp::new(
                bricks_height_texture_id,
                0.02,
                vector![1.0 / 25.0, 1.0 / 25.0],
            ),
            &MicrofacetDiffuseReflectionComp,
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ReferenceFrameComp::new(
                Point3::new(-25.0, 5.0, 0.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
                    * Orientation::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
                50.0,
            ),
            &AlbedoTextureComp(bricks_color_texture_id),
            &SpecularReflectanceComp(vector![1.0, 1.0, 1.0] * 0.02),
            &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
            &ParallaxMapComp::new(
                bricks_height_texture_id,
                0.02,
                vector![1.0 / 25.0, 1.0 / 25.0],
            ),
            &MicrofacetDiffuseReflectionComp,
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &ReferenceFrameComp::new(
                Point3::new(0.0, 5.0, 25.0),
                Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
                50.0,
            ),
            &AlbedoTextureComp(bricks_color_texture_id),
            &SpecularReflectanceComp(vector![1.0, 1.0, 1.0] * 0.02),
            &RoughnessTextureComp::unscaled(bricks_roughness_texture_id),
            &ParallaxMapComp::new(
                bricks_height_texture_id,
                0.02,
                vector![1.0 / 25.0, 1.0 / 25.0],
            ),
            &MicrofacetDiffuseReflectionComp,
            &MicrofacetSpecularReflectionComp,
        ))
        .unwrap();

    world
        .create_entity((
            &SphereMeshComp::new(25),
            &ReferenceFrameComp::unoriented_scaled(Point3::new(0.0, 15.0, 2.0), 0.7),
            &AlbedoComp(Vector3::zeros()),
            &EmissiveLuminanceComp(vector![1.0, 1.0, 1.0] * 1e5),
            &OmnidirectionalEmissionComp::new(vector![1.0, 1.0, 1.0] * 2e7, 0.7),
        ))
        .unwrap();

    world
        .create_entity(&UnidirectionalEmissionComp::new(
            vector![1.0, 1.0, 1.0] * 100000.0,
            UnitVector3::new_normalize(vector![0.6, -0.3, 1.0]),
            Degrees(2.0),
        ))
        .unwrap();

    world
        .create_entity(&AmbientEmissionComp::new(vector![1.0, 1.0, 1.0] * 5000.0))
        .unwrap();

    // world
    //     .create_entity((
    //         &VoxelSphereComp::new(500, 4),
    //         &VoxelTypeComp::new(VoxelType::Default),
    //         &ReferenceFrameComp::unoriented(point![-100.0, -100.0, -4.0]),
    //     ))
    //     .unwrap();

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
            &AlbedoComp(vector![0.8, 0.1, 0.1]),
        ))
        .unwrap();

    let cube_body_entity = world
        .create_entity((
            &BoxMeshComp::UNIT_CUBE,
            &UniformRigidBodyComp { mass_density: mass },
            &ReferenceFrameComp::for_unoriented_rigid_body(mass_position),
            &VelocityComp::stationary(),
            &AlbedoComp(vector![0.1, 0.1, 0.7]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 80.0),
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
            &AlbedoComp(vector![0.1, 0.7, 0.1]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 80.0),
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
            &AlbedoComp(vector![0.1, 0.1, 0.7]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 80.0),
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
            &AlbedoComp(vector![0.1, 0.1, 0.7]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 80.0),
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
            &AlbedoComp(vector![0.1, 0.1, 0.7]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 80.0),
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
            &AlbedoComp(vector![0.1, 0.1, 0.7]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 80.0),
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
            &AlbedoComp(vector![0.7, 0.1, 0.1]),
            &SpecularReflectanceComp::in_range_of(SpecularReflectanceComp::PLASTIC, 80.0),
            &UniformGravityComp::earth(),
        ))
        .unwrap();
}
