//! Running an event loop.

#![allow(unused)]
use super::{geometry::Degrees, gpu::rendering::RenderingSystem};
use crate::{
    application::Application,
    assets::Assets,
    camera::components::PerspectiveCameraComp,
    control::{
        motion::{SemiDirectionalMotionController, components::MotionControlComp},
        orientation::{RollFreeCameraOrientationController, components::OrientationControlComp},
    },
    game_loop::{GameLoop, GameLoopConfig},
    geometry::{Plane, Sphere},
    gpu::{
        self,
        rendering::RenderingConfig,
        texture::{ColorSpace, SamplerConfig, TextureAddressingConfig, TextureConfig, TextureID},
    },
    light::components::{
        AmbientEmissionComp, OmnidirectionalEmissionComp, ShadowableOmnidirectionalEmissionComp,
        ShadowableUnidirectionalEmissionComp, UnidirectionalEmissionComp,
    },
    material::{
        self, MaterialLibrary,
        components::{
            NormalMapComp, ParallaxMapComp, TexturedColorComp, TexturedRoughnessComp,
            UniformColorComp, UniformEmissiveLuminanceComp, UniformMetalnessComp,
            UniformRoughnessComp, UniformSpecularReflectanceComp,
        },
    },
    mesh::{
        FrontFaceSide, MeshRepository,
        components::{
            BoxMeshComp, ConeMeshComp, CylinderMeshComp, RectangleMeshComp, SphereMeshComp,
        },
        texture_projection::components::PlanarTextureProjectionComp,
    },
    model::InstanceFeatureManager,
    num::Float,
    physics::{
        PhysicsSimulator, SimulatorConfig,
        collision::{
            CollidableKind,
            components::{PlaneCollidableComp, SphereCollidableComp, VoxelObjectCollidableComp},
        },
        constraint::solver::ConstraintSolverConfig,
        fph,
        material::{ContactResponseParameters, components::UniformContactResponseComp},
        medium::UniformMedium,
        motion::{
            AngularVelocity, Orientation, Position,
            analytical::{
                constant_rotation::components::ConstantRotationComp,
                harmonic_oscillation::components::HarmonicOscillatorTrajectoryComp,
            },
            components::{
                LogsKineticEnergy, LogsMomentum, ReferenceFrameComp, Static, VelocityComp,
            },
        },
        rigid_body::{
            components::UniformRigidBodyComp,
            forces::{
                detailed_drag::components::DetailedDragComp,
                spring::{Spring, components::SpringComp},
                uniform_gravity::components::UniformGravityComp,
            },
        },
    },
    scene::{
        Scene, SceneEntityFlags,
        components::{ParentComp, SceneEntityFlagsComp, SceneGraphGroupComp, UncullableComp},
    },
    skybox::Skybox,
    util::bounds::UpperExclusiveBounds,
    voxel::{
        VoxelManager,
        components::{
            GradientNoiseVoxelTypesComp, MultifractalNoiseModificationComp, SameVoxelTypeComp,
            VoxelAbsorbingCapsuleComp, VoxelAbsorbingSphereComp, VoxelBoxComp,
            VoxelGradientNoisePatternComp, VoxelSphereComp, VoxelSphereUnionComp,
        },
        voxel_types::{FixedVoxelMaterialProperties, VoxelType, VoxelTypeRegistry},
    },
    window::{GameHandler, InputHandler, KeyActionMap, MouseButtonInputHandler, Window},
};
use anyhow::Result;
use nalgebra::{Point3, UnitVector3, Vector3, point, vector};
use rand::{Rng, SeedableRng, rngs::ThreadRng};
use std::{borrow::Cow, f64::consts::PI, path::PathBuf, sync::Arc};

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
    let (app, mouse_button_input_handler) = init_physics_lab(window)?;
    let input_handler = InputHandler::new(KeyActionMap::default(), mouse_button_input_handler);
    GameLoop::new(app, input_handler, GameLoopConfig::default())
}

fn init_app(window: Window) -> Result<(Application, MouseButtonInputHandler)> {
    let rendering_config = RenderingConfig::default();

    let vertical_field_of_view = Degrees(70.0);

    let simulator = PhysicsSimulator::new(SimulatorConfig::default(), UniformMedium::vacuum())?;

    let motion_controller = SemiDirectionalMotionController::new(8.0, true);
    let orientation_controller =
        RollFreeCameraOrientationController::new(vertical_field_of_view, 1.0);

    let voxel_type_registry = VoxelTypeRegistry::new(
        vec![
            Cow::Borrowed("Ground"),
            Cow::Borrowed("Metal"),
            Cow::Borrowed("Snow"),
            Cow::Borrowed("Rock"),
            // Cow::Borrowed("Brick"),
            // Cow::Borrowed("Wood"),
        ],
        vec![1.0; 4],
        vec![
            FixedVoxelMaterialProperties::new(0.02, 1.0, 0.0, 0.0),
            FixedVoxelMaterialProperties::new(1.0, 1.0, 1.0, 0.0),
            FixedVoxelMaterialProperties::new(0.04, 1.0, 0.0, 0.0),
            FixedVoxelMaterialProperties::new(0.03, 1.0, 0.0, 0.0),
            // FixedVoxelMaterialProperties::new(0.02, 1.0, 0.0, 0.0),
            // FixedVoxelMaterialProperties::new(0.03, 1.0, 0.0, 0.0),
        ],
        vec![
            PathBuf::from("assets/Ground029_4K-JPG/Ground029_4K-JPG_Color.jpg"),
            PathBuf::from("assets/Metal062C_4K-JPG/Metal062C_4K-JPG_Color.jpg"),
            PathBuf::from("assets/Snow007A_4K-JPG/Snow007A_4K-JPG_Color.jpg"),
            PathBuf::from("assets/Rock022_4K-JPG/Rock022_4K-JPG_Color.jpg"),
            // PathBuf::from("assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Color.jpg"),
            // PathBuf::from("assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_Color.jpg"),
        ],
        vec![
            PathBuf::from("assets/Ground029_4K-JPG/Ground029_4K-JPG_Roughness.jpg"),
            PathBuf::from("assets/Metal062C_4K-JPG/Metal062C_4K-JPG_Roughness.jpg"),
            PathBuf::from("assets/Snow007A_4K-JPG/Snow007A_4K-JPG_Roughness.jpg"),
            PathBuf::from("assets/Rock022_4K-JPG/Rock022_4K-JPG_Roughness.jpg"),
            // PathBuf::from("assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Roughness.jpg"),
            // PathBuf::from("assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_Roughness.jpg"),
        ],
        vec![
            PathBuf::from("assets/Ground029_4K-JPG/Ground029_4K-JPG_NormalDX.jpg"),
            PathBuf::from("assets/Metal062C_4K-JPG/Metal062C_4K-JPG_NormalDX.jpg"),
            PathBuf::from("assets/Snow007A_4K-JPG/Snow007A_4K-JPG_NormalDX.jpg"),
            PathBuf::from("assets/Rock022_4K-JPG/Rock022_4K-JPG_NormalDX.jpg"),
            // PathBuf::from("assets/Bricks059_4K-JPG/Bricks059_4K-JPG_NormalDX.jpg"),
            // PathBuf::from("assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_NormalDX.jpg"),
        ],
    )
    .unwrap();

    let app = Application::new(
        Arc::new(window),
        rendering_config,
        simulator,
        Some(Box::new(motion_controller)),
        Some(Box::new(orientation_controller)),
        voxel_type_registry,
    )?;

    let mut mouse_button_input_handler = MouseButtonInputHandler::default();

    let mut assets = app.assets().write().unwrap();

    let skybox_texture_id = assets.load_cubemap_texture_from_paths(
        "assets/space_skybox/right.png",
        "assets/space_skybox/left.png",
        "assets/space_skybox/top.png",
        "assets/space_skybox/bottom.png",
        "assets/space_skybox/front.png",
        "assets/space_skybox/back.png",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            ..Default::default()
        },
        Some(SamplerConfig::default()),
    )?;

    // let bricks_color_texture_id = assets.load_texture_from_path(
    //     "assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Color.jpg",
    //     TextureConfig {
    //         color_space: ColorSpace::Srgb,
    //         ..Default::default()
    //     },
    //     Some(SamplerConfig {
    //         addressing: TextureAddressingConfig::REPEATING,
    //         ..Default::default()
    //     }),
    // )?;

    // let bricks_roughness_texture_id = assets.load_texture_from_path(
    //     "assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Roughness.jpg",
    //     TextureConfig {
    //         color_space: ColorSpace::Linear,
    //         ..Default::default()
    //     },
    //     Some(SamplerConfig {
    //         addressing: TextureAddressingConfig::REPEATING,
    //         ..Default::default()
    //     }),
    // )?;

    // let bricks_height_texture_id = assets.load_texture_from_path(
    //     "assets/Bricks059_4K-JPG/Bricks059_4K-JPG_Displacement.jpg",
    //     TextureConfig {
    //         color_space: ColorSpace::Linear,
    //         ..Default::default()
    //     },
    //     Some(SamplerConfig {
    //         addressing: TextureAddressingConfig::REPEATING,
    //         ..Default::default()
    //     }),
    // )?;

    // let wood_floor_color_texture_id = assets.load_texture_from_path(
    //     "assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_Color.jpg",
    //     TextureConfig {
    //         color_space: ColorSpace::Srgb,
    //         ..Default::default()
    //     },
    //     Some(SamplerConfig {
    //         addressing: TextureAddressingConfig::REPEATING,
    //         ..Default::default()
    //     }),
    // )?;

    // let wood_floor_roughness_texture_id = assets.load_texture_from_path(
    //     "assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_Roughness.jpg",
    //     TextureConfig {
    //         color_space: ColorSpace::Linear,
    //         ..Default::default()
    //     },
    //     Some(SamplerConfig {
    //         addressing: TextureAddressingConfig::REPEATING,
    //         ..Default::default()
    //     }),
    // )?;

    // let wood_floor_normal_texture_id = assets.load_texture_from_path(
    //     "assets/WoodFloor041_4K-JPG/WoodFloor041_4K-JPG_NormalDX.jpg",
    //     TextureConfig {
    //         color_space: ColorSpace::Linear,
    //         ..Default::default()
    //     },
    //     Some(SamplerConfig {
    //         addressing: TextureAddressingConfig::REPEATING,
    //         ..Default::default()
    //     }),
    // )?;

    drop(assets);

    app.set_skybox_for_current_scene(Skybox::new(skybox_texture_id, 2e3));

    let player_entity = app.create_entity((
        // &CylinderMeshComp::new(1.8, 0.25, 30),
        // &SphereMeshComp::new(15),
        // &UniformRigidBodyComp { mass_density: 1e3 },
        &ReferenceFrameComp::unscaled(
            // Point3::new(0.0, 7.0, -10.0),
            Point3::new(0.0, 0.0, 0.0),
            Orientation::from_axis_angle(&Vector3::y_axis(), PI),
        ),
        &VelocityComp::stationary(),
        &MotionControlComp::new(),
        &OrientationControlComp::new(),
        // &UniformGravityComp::downward(9.81),
        &SceneGraphGroupComp,
    ))?;

    app.create_entity((
        &ParentComp::new(player_entity),
        &PerspectiveCameraComp::new(
            vertical_field_of_view,
            UpperExclusiveBounds::new(0.01, 1000.0),
        ),
    ))?;

    let laser_entity = app.create_entity((
        &ParentComp::new(player_entity),
        &ReferenceFrameComp::unscaled(
            Point3::new(0.15, -0.3, 0.0),
            Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
        ),
        &CylinderMeshComp::new(150.0, 0.02, 16),
        &UniformColorComp(vector![0.9, 0.05, 0.05]),
        &UniformEmissiveLuminanceComp(1e6),
        &VoxelAbsorbingCapsuleComp::new(
            vector![0.0, 0.0, 0.0],
            vector![0.0, 100.0, 0.0],
            0.3,
            300.0,
        ),
        &SceneEntityFlagsComp(SceneEntityFlags::IS_DISABLED | SceneEntityFlags::CASTS_NO_SHADOWS),
    ))?;

    mouse_button_input_handler.left_pressed =
        Some(Box::new(move |app| app.enable_scene_entity(&laser_entity)));
    mouse_button_input_handler.left_released =
        Some(Box::new(move |app| app.disable_scene_entity(&laser_entity)));

    let absorbing_sphere_entity = app.create_entity((
        &ParentComp::new(player_entity),
        &ReferenceFrameComp::unoriented_scaled(Point3::new(0.0, 0.0, -3.0), 0.1),
        &SphereMeshComp::new(64),
        &UniformColorComp(vector![0.9, 0.05, 0.05]),
        &UniformEmissiveLuminanceComp(1e6),
        &ShadowableOmnidirectionalEmissionComp::new(vector![1.0, 0.2, 0.2] * 1e5, 0.2),
        &VoxelAbsorbingSphereComp::new(vector![0.0, 0.0, 0.0], 10.0, 15.0),
        &SceneEntityFlagsComp(SceneEntityFlags::IS_DISABLED),
    ))?;

    mouse_button_input_handler.right_pressed = Some(Box::new(move |app| {
        app.enable_scene_entity(&absorbing_sphere_entity)
    }));
    mouse_button_input_handler.right_released = Some(Box::new(move |app| {
        app.disable_scene_entity(&absorbing_sphere_entity)
    }));

    // app.create_entity((
    //     &app.load_mesh_from_obj_file("assets/Dragon_1.obj")?,
    //     &ReferenceFrameComp::new(
    //         Point3::new(0.0, 1.5, 11.0),
    //         Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
    //         0.06,
    //     ),
    //     &UniformColorComp(vector![0.1, 0.2, 0.6]),
    //     &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 50.0),
    //     &UniformRoughnessComp(0.4),
    // ))?;

    // app.create_entity((
    //     &CylinderMeshComp::new(10.0, 0.6, 100),
    //     &ReferenceFrameComp::unoriented(Point3::new(7.0, 0.5, 5.0)),
    //     &UniformColorComp::IRON,
    //     &UniformSpecularReflectanceComp::METAL,
    //     &UniformMetalnessComp::METAL,
    //     &UniformRoughnessComp(0.5),
    // ))?;

    // app.create_entity((
    //     &app.load_mesh_from_obj_file("assets/abstract_object.obj")?,
    //     &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(7.0, 7.7,
    // 5.0), 0.02),     &ConstantRotationComp::new(
    //         0.0,
    //         Orientation::from_axis_angle(&Vector3::y_axis(), 0.0),
    //         AngularVelocity::new(Vector3::y_axis(), Degrees(50.0)),
    //     ),
    //     &UniformColorComp::COPPER,
    //     &UniformSpecularReflectanceComp::METAL,
    //     &UniformMetalnessComp::METAL,
    //     &UniformRoughnessComp(0.35),
    // ))?;

    // app.create_entity((
    //     &app.load_mesh_from_obj_file("assets/abstract_pyramid.obj")?,
    //     &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(-1.0, 9.0,
    // 9.0), 0.035),     &ConstantRotationComp::new(
    //         0.0,
    //         Orientation::from_axis_angle(&Vector3::x_axis(), 0.4),
    //         AngularVelocity::new(Vector3::y_axis(), Degrees(-60.0)),
    //     ),
    //     &UniformColorComp(vector![0.7, 0.3, 0.2]),
    //     &UniformRoughnessComp(0.95),
    // ))?;

    // app.create_entity((
    //     &BoxMeshComp::UNIT_CUBE,
    //     &ReferenceFrameComp::unoriented_scaled(Point3::new(-9.0, -1.0, 5.0),
    // 2.0),     &UniformColorComp(vector![0.1, 0.7, 0.3]),
    //     &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 0.0),
    //     &UniformRoughnessComp(0.55),
    // ))?;

    // app.create_entity((
    //     &SphereMeshComp::new(100),
    //     &ReferenceFrameComp::unoriented_scaled(Point3::new(-9.0, 2.0, 5.0), 4.0),
    //     &UniformColorComp(vector![0.3, 0.2, 0.7]),
    //     &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::STONE, 0.5),
    //     &UniformRoughnessComp(0.7),
    // ))?;

    // app.create_entity((
    //     &app.load_mesh_from_obj_file("assets/abstract_cube.obj")?,
    //     &ReferenceFrameComp::for_scaled_driven_rotation(Point3::new(-9.0, 5.8,
    // 5.0), 0.016),     &ConstantRotationComp::new(
    //         0.0,
    //         Orientation::from_axis_angle(&Vector3::y_axis(), 0.7),
    //         AngularVelocity::new(Vector3::x_axis(), Degrees(30.0)),
    //     ),
    //     &UniformColorComp::GOLD,
    //     &UniformSpecularReflectanceComp::METAL,
    //     &UniformMetalnessComp::METAL,
    //     &UniformRoughnessComp(0.4),
    // ))?;

    // app.create_entity((
    //     &RectangleMeshComp::UNIT_SQUARE,
    //     // &PlanarTextureProjectionComp::for_rectangle(&
    // RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),     &ReferenceFrameComp::new(
    //         Point3::new(0.0, -20.0, 0.0),
    //         Orientation::from_axis_angle(&Vector3::z_axis(), 0.0),
    //         // 50.0,
    //         500.0,
    //     ),
    //     // &TexturedColorComp(wood_floor_color_texture_id),
    //     &UniformColorComp(vector![1.0, 1.0, 1.0]),
    //     &UniformSpecularReflectanceComp::in_range_of(
    //         UniformSpecularReflectanceComp::LIVING_TISSUE,
    //         100.0,
    //     ),
    //     // &TexturedRoughnessComp::unscaled(wood_floor_roughness_texture_id),
    //     // &NormalMapComp(wood_floor_normal_texture_id),
    // ))?;

    // app.create_entity((
    //     &RectangleMeshComp::UNIT_SQUARE,
    //     &PlanarTextureProjectionComp::for_rectangle(&
    // RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),     &ReferenceFrameComp::new(
    //         Point3::new(25.0, 5.0, 0.0),
    //         Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
    //             * Orientation::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
    //         50.0,
    //     ),
    //     &TexturedColorComp(bricks_color_texture_id),
    //     &UniformSpecularReflectanceComp(0.02),
    //     &TexturedRoughnessComp::unscaled(bricks_roughness_texture_id),
    //     &ParallaxMapComp::new(
    //         bricks_height_texture_id,
    //         0.02,
    //         vector![1.0 / 25.0, 1.0 / 25.0],
    //     ),
    // ))?;

    // app.create_entity((
    //     &RectangleMeshComp::UNIT_SQUARE,
    //     &PlanarTextureProjectionComp::for_rectangle(&
    // RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),     &ReferenceFrameComp::new(
    //         Point3::new(-25.0, 5.0, 0.0),
    //         Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0)
    //             * Orientation::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
    //         50.0,
    //     ),
    //     &TexturedColorComp(bricks_color_texture_id),
    //     &UniformSpecularReflectanceComp(0.02),
    //     &TexturedRoughnessComp::unscaled(bricks_roughness_texture_id),
    //     &ParallaxMapComp::new(
    //         bricks_height_texture_id,
    //         0.02,
    //         vector![1.0 / 25.0, 1.0 / 25.0],
    //     ),
    // ))?;

    // app.create_entity((
    //     &RectangleMeshComp::UNIT_SQUARE,
    //     &PlanarTextureProjectionComp::for_rectangle(&
    // RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),     &ReferenceFrameComp::new(
    //         Point3::new(0.0, 5.0, 25.0),
    //         Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
    //         50.0,
    //     ),
    //     &TexturedColorComp(bricks_color_texture_id),
    //     &UniformSpecularReflectanceComp(0.02),
    //     &TexturedRoughnessComp::unscaled(bricks_roughness_texture_id),
    //     &ParallaxMapComp::new(
    //         bricks_height_texture_id,
    //         0.02,
    //         vector![1.0 / 25.0, 1.0 / 25.0],
    //     ),
    // ))?;

    app.create_entity((
        &SphereMeshComp::new(25),
        // &ReferenceFrameComp::unoriented_scaled(Point3::new(0.0, 15.0, 2.0), 0.7),
        &ReferenceFrameComp::unoriented_scaled(Point3::new(20.0, 0.0, 20.0), 1.0),
        &UniformColorComp(vector![1.0, 1.0, 1.0]),
        &UniformEmissiveLuminanceComp(1e6),
        &ShadowableOmnidirectionalEmissionComp::new(vector![1.0, 1.0, 1.0] * 2e7, 0.7),
    ))?;

    // app.create_entity(&ShadowableUnidirectionalEmissionComp::new(
    //     vector![1.0, 1.0, 1.0] * 10000.0,
    //     UnitVector3::new_normalize(vector![0.6, -0.3, 1.0]),
    //     Degrees(2.0),
    // ))?;

    app.create_entity(&AmbientEmissionComp::new(vector![1.0, 1.0, 1.0] * 1000.0))?;

    // TODO: Check why this crashes
    // app.create_entity((
    //     &VoxelSphereComp::new(800),
    //     // &VoxelGradientNoisePatternComp::new(500, 500, 500, 3.0, 0.3, 1),
    //     &VoxelTypeComp::new(VoxelType::Default, 0.1),
    //     &ReferenceFrameComp::unoriented(point![25.0, -25.0, -15.0]),
    // ))?;

    // app.create_entity((
    //     // &VoxelSphereComp::new(0.25, 20.0),
    //     // &VoxelBoxComp::new(0.25, 4.0, 2.0, 1.0),
    //     // &VoxelGradientNoisePatternComp::new(0.5, 50.0, 50.0, 50.0, 2e-2,0.3,0),
    //     &VoxelSphereUnionComp::new(0.25, 10.0, 10.0, [20.0, 0.0, 0.0], 5.0),
    //     // &SameVoxelTypeComp::new(VoxelType::from_idx(0)),
    //     &GradientNoiseVoxelTypesComp::new(["Ground", "Rock", "Metal"], 6e-2, 1.0, 0),
    //     &MultifractalNoiseModificationComp::new(8, 0.02, 2.0, 0.6, 4.0, 0),
    //     // &GradientNoiseVoxelTypesComp::new(["Snow", "Rock"], 6e-2,1.0,0),     // &ReferenceFrameComp::unoriented(point![0.0, 0.0, 5.0]),
    //     &ReferenceFrameComp::unoriented(point![0.0, 0.0, 20.0]),
    //     &VelocityComp::angular(AngularVelocity::new(Vector3::y_axis(), Degrees(20.0))),
    // ))?;

    // app.create_entity((
    //     &VoxelSphereComp::new(0.25, 50.0),
    //     // &GradientNoiseVoxelTypesComp::new(["Ground", "Rock", "Metal"], 6e-2, 1.0, 1),
    //     &SameVoxelTypeComp::new(VoxelType::from_idx(1)),
    //     // &MultifractalNoiseModificationComp::new(8, 0.02, 2.0, 0.6, 4.0, 1),
    //     &ReferenceFrameComp::unoriented(point![0.0, 0.0, 20.0]),
    // ))?;

    // create_harmonic_oscillation_experiment(&app, Point3::new(0.0, 10.0, 2.0), 1.0, 10.0, 3.0);
    create_free_rotation_experiment(&app, Point3::new(0.0, 7.0, 2.0), 5.0, 1e-3);
    // create_drag_drop_experiment(&app, Point3::new(0.0, 20.0, 4.0));

    Ok((app, mouse_button_input_handler))
}

fn init_physics_lab(window: Window) -> Result<(Application, MouseButtonInputHandler)> {
    let rendering_config = RenderingConfig::default();

    let vertical_field_of_view = Degrees(70.0);

    let simulator = PhysicsSimulator::new(
        SimulatorConfig {
            n_substeps: 1,
            match_frame_duration: false,
            constraint_solver_config: Some(ConstraintSolverConfig {
                n_iterations: 8,
                old_impulse_weight: 0.4,
                n_positional_correction_iterations: 3,
                positional_correction_factor: 0.2,
            }),
            ..SimulatorConfig::default()
        },
        UniformMedium::vacuum(),
    )?;

    let motion_controller = SemiDirectionalMotionController::new(8.0, true);
    let orientation_controller =
        RollFreeCameraOrientationController::new(vertical_field_of_view, 1.0);

    let voxel_type_registry = VoxelTypeRegistry::new(
        vec![Cow::Borrowed("Metal")],
        vec![1e9; 1],
        vec![FixedVoxelMaterialProperties::new(1.0, 1.0, 1.0, 0.0)],
        vec![PathBuf::from(
            "assets/Metal062C_4K-JPG/Metal062C_4K-JPG_Color.jpg",
        )],
        vec![PathBuf::from(
            "assets/Metal062C_4K-JPG/Metal062C_4K-JPG_Roughness.jpg",
        )],
        vec![PathBuf::from(
            "assets/Metal062C_4K-JPG/Metal062C_4K-JPG_NormalDX.jpg",
        )],
    )
    .unwrap();

    let app = Application::new(
        Arc::new(window),
        rendering_config,
        simulator,
        Some(Box::new(motion_controller)),
        Some(Box::new(orientation_controller)),
        voxel_type_registry,
    )?;

    let mouse_button_input_handler = MouseButtonInputHandler::default();

    let mut assets = app.assets().write().unwrap();

    let concrete_color_texture_id = assets.load_texture_from_path(
        "assets/Concrete034_4K-JPG/Concrete034_4K-JPG_Color.jpg",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            ..Default::default()
        },
        Some(SamplerConfig {
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        }),
    )?;
    let concrete_roughness_texture_id = assets.load_texture_from_path(
        "assets/Concrete034_4K-JPG/Concrete034_4K-JPG_Roughness.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            ..Default::default()
        },
        Some(SamplerConfig {
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        }),
    )?;
    let concrete_normal_texture_id = assets.load_texture_from_path(
        "assets/Concrete034_4K-JPG/Concrete034_4K-JPG_NormalDX.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            ..Default::default()
        },
        Some(SamplerConfig {
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        }),
    )?;

    let plastic_color_texture_id = assets.load_texture_from_path(
        "assets/Plastic007_4K-JPG/Plastic007_4K-JPG_Color.jpg",
        TextureConfig {
            color_space: ColorSpace::Srgb,
            ..Default::default()
        },
        Some(SamplerConfig {
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        }),
    )?;

    let plastic_roughness_texture_id = assets.load_texture_from_path(
        "assets/Plastic007_4K-JPG/Plastic007_4K-JPG_Roughness.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            ..Default::default()
        },
        Some(SamplerConfig {
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        }),
    )?;

    let plastic_normal_texture_id = assets.load_texture_from_path(
        "assets/Plastic007_4K-JPG/Plastic007_4K-JPG_NormalDX.jpg",
        TextureConfig {
            color_space: ColorSpace::Linear,
            ..Default::default()
        },
        Some(SamplerConfig {
            addressing: TextureAddressingConfig::REPEATING,
            ..Default::default()
        }),
    )?;

    drop(assets);

    app.create_entity((
        &ReferenceFrameComp::unscaled(
            Point3::new(0.0, 0.0, -5.0),
            Orientation::from_axis_angle(&Vector3::y_axis(), PI),
        ),
        &VelocityComp::stationary(),
        &MotionControlComp::new(),
        &OrientationControlComp::new(),
        &PerspectiveCameraComp::new(
            vertical_field_of_view,
            UpperExclusiveBounds::new(0.01, 1000.0),
        ),
    ))?;

    let sphere_radius = 0.5;
    let n_y = 4;
    let room_extent = 8.0;
    let n_spheres_y = 2 * n_y + 1;

    create_spheres(
        &app,
        sphere_radius,
        [3, n_y, 3],
        point![
            0.0,
            fph::from(n_spheres_y) * sphere_radius - room_extent + 2.0,
            0.0
        ],
        false,
        plastic_color_texture_id,
        plastic_roughness_texture_id,
        plastic_normal_texture_id,
    )?;

    create_room(
        &app,
        room_extent,
        20.0,
        concrete_color_texture_id,
        concrete_roughness_texture_id,
        concrete_normal_texture_id,
    )?;

    let voxel_extent = 0.25;
    let box_size = 6.0;
    app.create_entity((
        &VoxelBoxComp::new(voxel_extent, box_size, box_size, box_size),
        &SameVoxelTypeComp::new(VoxelType::from_idx(0)),
        &ReferenceFrameComp::unoriented(point![
            0.0,
            0.5 * voxel_extent * box_size - 0.5 * room_extent,
            0.0
        ]),
        &VelocityComp::angular(AngularVelocity::new(Vector3::y_axis(), Degrees(500.0))),
        &VoxelObjectCollidableComp::new(CollidableKind::Static),
        // &Static,
    ))?;

    app.create_entity(&ShadowableUnidirectionalEmissionComp::new(
        vector![1.0, 1.0, 1.0] * 200000.0,
        UnitVector3::new_normalize(vector![0.0, -1.0, 0.0]),
        Degrees(2.0),
    ))?;

    app.create_entity(&AmbientEmissionComp::new(
        vector![1.0, 1.0, 1.0] * 2000000.0,
    ))?;

    Ok((app, mouse_button_input_handler))
}

fn create_spheres(
    app: &Application,
    radius: fph,
    n: [u32; 3],
    center: Position,
    alternate_shift: bool,
    color_texture_id: TextureID,
    roughness_texture_id: TextureID,
    normal_texture_id: TextureID,
) -> Result<()> {
    let mut x = center.x - fph::from(n[0]) * 2.0 * radius;
    for i in 0..(2 * n[0] + 1) {
        let mut y = center.y - fph::from(n[1]) * 2.0 * radius;
        if alternate_shift && i % 2 == 0 {
            y += radius;
        };
        for j in 0..(2 * n[1] + 1) {
            let mut z = center.z - fph::from(n[2]) * 2.0 * radius;
            if alternate_shift && j % 2 == 0 {
                z += radius;
            };
            for k in 0..(2 * n[2] + 1) {
                app.create_entity((
                    &SphereMeshComp::new(100),
                    &ReferenceFrameComp::unoriented_scaled(
                        Point3::new(x * radius, y * radius, z * radius),
                        radius,
                    ),
                    &VelocityComp::linear(vector![0.0, 0.0, 0.0]),
                    &UniformRigidBodyComp { mass_density: 1.0 },
                    &UniformContactResponseComp(ContactResponseParameters {
                        restitution_coef: 0.7,
                        static_friction_coef: 0.5,
                        dynamic_friction_coef: 0.3,
                    }),
                    &SphereCollidableComp::new(
                        CollidableKind::Dynamic,
                        &Sphere::new(Point3::origin(), radius),
                    ),
                    &UniformGravityComp::earth(),
                    &TexturedColorComp(color_texture_id),
                    &UniformSpecularReflectanceComp::in_range_of(
                        UniformSpecularReflectanceComp::PLASTIC,
                        0.0,
                    ),
                    &TexturedRoughnessComp::unscaled(roughness_texture_id),
                    &NormalMapComp(normal_texture_id),
                    &PlanarTextureProjectionComp::for_rectangle(
                        &RectangleMeshComp::UNIT_SQUARE,
                        0.2,
                        0.2,
                    ),
                    // &LogsKineticEnergy,
                    // &LogsMomentum,
                ))?;
                z += 2.0 * radius;
            }
            y += 2.0 * radius;
        }
        x += 2.0 * radius;
    }
    Ok(())
}

fn create_room(
    app: &Application,
    extent: fph,
    angular_speed: fph,
    color_texture_id: TextureID,
    roughness_texture_id: TextureID,
    normal_texture_id: TextureID,
) -> Result<()> {
    let half_extent = 0.5 * extent;
    let angular_velocity = AngularVelocity::new(Vector3::z_axis(), Degrees(angular_speed));

    for (position, orientation) in [
        (
            Point3::new(0.0, half_extent, 0.0),
            Orientation::from_axis_angle(&Vector3::x_axis(), 0.0),
        ),
        (
            Point3::new(0.0, half_extent, 0.0),
            Orientation::from_axis_angle(&Vector3::x_axis(), PI),
        ),
        (
            Point3::new(0.0, half_extent, 0.0),
            Orientation::from_axis_angle(&Vector3::z_axis(), PI / 2.0),
        ),
        (
            Point3::new(0.0, half_extent, 0.0),
            Orientation::from_axis_angle(&Vector3::z_axis(), -PI / 2.0),
        ),
        (
            Point3::new(0.0, half_extent, 0.0),
            Orientation::from_axis_angle(&Vector3::x_axis(), PI / 2.0),
        ),
        (
            Point3::new(0.0, half_extent, 0.0),
            Orientation::from_axis_angle(&Vector3::x_axis(), -PI / 2.0),
        ),
    ] {
        let mut frame = ReferenceFrameComp::scaled_with_offset_origin(
            position.coords / extent,
            Point3::origin(),
            orientation,
            extent,
        );
        let wall = app.create_entity((
            &RectangleMeshComp::UNIT_SQUARE,
            &frame,
            &ConstantRotationComp::new(0.0, orientation, angular_velocity),
            &VelocityComp::angular(angular_velocity),
            &UniformContactResponseComp(ContactResponseParameters {
                restitution_coef: 0.2,
                static_friction_coef: 0.7,
                dynamic_friction_coef: 0.5,
            }),
            &PlaneCollidableComp::new(CollidableKind::Static, &Plane::new(Vector3::y_axis(), 0.0)),
            &TexturedColorComp(color_texture_id),
            &UniformSpecularReflectanceComp(0.01),
            &TexturedRoughnessComp::unscaled(roughness_texture_id),
            &NormalMapComp(normal_texture_id),
            &PlanarTextureProjectionComp::for_rectangle(&RectangleMeshComp::UNIT_SQUARE, 2.0, 2.0),
            &SceneGraphGroupComp,
        ))?;

        for x in [-0.4, 0.4] {
            for z in [-0.4, 0.4] {
                app.create_entity((
                    &ParentComp::new(wall),
                    // &SphereMeshComp::new(25),
                    &ReferenceFrameComp::unoriented_scaled(Point3::new(x, 0.1, z), 0.2 / extent),
                    // &UniformColorComp(vector![1.0, 1.0, 1.0]),
                    // &UniformEmissiveLuminanceComp(1e6),
                    &OmnidirectionalEmissionComp::new(vector![1.0, 1.0, 1.0] * 1e7, 0.7),
                ))?;
            }
        }
    }
    Ok(())
}

fn create_harmonic_oscillation_experiment(
    app: &Application,
    position: Position,
    mass: fph,
    spring_constant: fph,
    amplitude: fph,
) -> Result<()> {
    let angular_frequency = fph::sqrt(spring_constant / mass);
    let period = fph::TWO_PI / angular_frequency;

    let attachment_position = position;
    let mass_position = attachment_position + vector![0.0, -2.0 * amplitude - 0.5, 0.0];

    let reference_position = attachment_position + vector![-2.0, -amplitude - 0.5, 0.0];

    let attachment_point_entity = app.create_entity((
        &SphereMeshComp::new(15),
        &ReferenceFrameComp::unoriented_scaled(attachment_position, 0.2),
        &UniformColorComp(vector![0.8, 0.1, 0.1]),
    ))?;

    let cube_body_entity = app.create_entity((
        &BoxMeshComp::UNIT_CUBE,
        &UniformRigidBodyComp { mass_density: mass },
        &ReferenceFrameComp::for_unoriented_rigid_body(mass_position),
        &VelocityComp::stationary(),
        &UniformColorComp(vector![0.1, 0.1, 0.7]),
        &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 80.0),
        &LogsKineticEnergy,
        &LogsMomentum,
    ))?;

    app.create_entity((
        &ReferenceFrameComp::default(),
        &SpringComp::new(
            attachment_point_entity,
            cube_body_entity,
            Position::origin(),
            Position::origin(),
            Spring::standard(spring_constant, 0.0, amplitude + 0.5),
        ),
    ))?;

    app.create_entity((
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
        &UniformColorComp(vector![0.1, 0.7, 0.1]),
        &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 80.0),
    ))?;

    Ok(())
}

fn create_free_rotation_experiment(
    app: &Application,
    position: Position,
    angular_speed: fph,
    angular_velocity_perturbation_fraction: fph,
) -> Result<()> {
    let major_axis_body_position = position + vector![5.0, 0.0, 0.0];
    let intermediate_axis_body_position = position;
    let minor_axis_body_position = position - vector![5.0, 0.0, 0.0];

    let angular_velocity_perturbation = angular_speed * angular_velocity_perturbation_fraction;

    app.create_entity((
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
        &UniformColorComp(vector![0.1, 0.1, 0.7]),
        &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 80.0),
        &LogsKineticEnergy,
        &LogsMomentum,
    ))?;

    app.create_entity((
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
        &UniformColorComp(vector![0.1, 0.1, 0.7]),
        &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 80.0),
        &LogsKineticEnergy,
        &LogsMomentum,
    ))?;

    app.create_entity((
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
        &UniformColorComp(vector![0.1, 0.1, 0.7]),
        &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 80.0),
        &LogsKineticEnergy,
        &LogsMomentum,
    ))?;

    Ok(())
}

fn create_drag_drop_experiment(app: &Application, position: Position) -> Result<()> {
    app.simulator()
        .write()
        .unwrap()
        .set_medium(UniformMedium::moving_air(vector![0.0, 3.0, 0.0]));

    app.create_entity((
        // &SphereMeshComp::new(100),
        &ConeMeshComp::new(2.0, 1.0, 100),
        // &BoxMeshComp::new(3.0, 0.4, 1.0, FrontFaceSide::Outside),
        &UniformRigidBodyComp { mass_density: 10.0 },
        &ReferenceFrameComp::for_rigid_body(
            position,
            Orientation::from_axis_angle(&Vector3::z_axis(), 3.0),
        ),
        &VelocityComp::angular(AngularVelocity::zero()),
        &UniformColorComp(vector![0.1, 0.1, 0.7]),
        &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 80.0),
        &UniformGravityComp::earth(),
        &DetailedDragComp::new(1.0),
        &LogsKineticEnergy,
        &LogsMomentum,
    ))?;

    app.create_entity((
        &ConeMeshComp::new(2.0, 1.0, 100),
        &UniformRigidBodyComp { mass_density: 10.0 },
        &ReferenceFrameComp::for_rigid_body(
            position + vector![-5.0, 0.0, 0.0],
            Orientation::from_axis_angle(&Vector3::z_axis(), 3.0),
        ),
        &VelocityComp::angular(AngularVelocity::zero()),
        &UniformColorComp(vector![0.7, 0.1, 0.1]),
        &UniformSpecularReflectanceComp::in_range_of(UniformSpecularReflectanceComp::PLASTIC, 80.0),
        &UniformGravityComp::earth(),
    ))?;

    Ok(())
}
