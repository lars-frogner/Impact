//! `LockableResource` implementations for Engine types.

use crate::declare_lockable_resource;

// ============================================================================
// GAME LOOP (0-9)
// ============================================================================

declare_lockable_resource!(crate::game_loop::GameLoopController, 0);

// ============================================================================
// INPUT MANAGER (10-49)
// ============================================================================

declare_lockable_resource!(crate::input::InputManager, 10);

// ============================================================================
// ENTITIES (50-99)
// ============================================================================

declare_lockable_resource!(impact_id::EntityIDManager, 50);
declare_lockable_resource!(impact_ecs::world::EntityStager, 60);
declare_lockable_resource!(impact_ecs::world::World, 70);

// ============================================================================
// RESOURCES (100-199)
// ============================================================================

declare_lockable_resource!(crate::resource::ResourceManager, 100);

// ============================================================================
// SCENE (200-299)
// ============================================================================

declare_lockable_resource!(crate::scene::Scene, 200);

// Scene components (must be acquired after Scene itself)
declare_lockable_resource!(Option<impact_scene::skybox::Skybox>, 210);
declare_lockable_resource!(impact_camera::CameraManager, 220);
declare_lockable_resource!(impact_light::LightManager, 230);
declare_lockable_resource!(impact_particle::ParticleEmitterManager, 240);
declare_lockable_resource!(impact_voxel::VoxelManager, 250);
declare_lockable_resource!(impact_scene::model::ModelInstanceManager, 260);
declare_lockable_resource!(impact_intersection::IntersectionManager, 270);
declare_lockable_resource!(impact_scene::graph::SceneGraph, 280);

// ============================================================================
// PHYSICS (300-399)
// ============================================================================

declare_lockable_resource!(crate::physics::PhysicsSimulator, 300);

// PhysicsSimulator components (must be acquired after PhysicsSimulator itself)
// Order follows natural physics simulation pipeline:
// Bodies → Anchors → Forces → Motion → Constraints → Collisions
declare_lockable_resource!(impact_physics::rigid_body::RigidBodyManager, 310);
declare_lockable_resource!(impact_physics::anchor::AnchorManager, 320);
declare_lockable_resource!(impact_physics::force::ForceGeneratorManager, 330);
declare_lockable_resource!(impact_physics::driven_motion::MotionDriverManager, 340);
declare_lockable_resource!(impact_physics::constraint::ConstraintManager, 350);
declare_lockable_resource!(impact_voxel::collidable::CollisionWorld, 360);

// ============================================================================
// GPU (400-499)
// ============================================================================

declare_lockable_resource!(crate::gpu::GraphicsSystem, 400);

declare_lockable_resource!(crate::physics::gpu::GPUSimulationManager, 410);

declare_lockable_resource!(crate::rendering::RenderingSystem, 420);

// GraphicsSystem components (must be acquired after GraphicsSystem and RenderingSystem)
declare_lockable_resource!(impact_gpu::timestamp_query::TimestampQueryManager, 430);
declare_lockable_resource!(impact_gpu::shader::ShaderManager, 440);
declare_lockable_resource!(crate::gpu::resource::GPUResourceManager, 450);
declare_lockable_resource!(impact_gpu::resource_group::GPUResourceGroupManager, 460);
declare_lockable_resource!(impact_gpu::storage::StorageGPUBufferManager, 470);

// RenderingSystem components (must be acquired after GraphicsSystem components)
declare_lockable_resource!(
    impact_rendering::attachment::RenderAttachmentTextureManager,
    480
);
declare_lockable_resource!(impact_rendering::postprocessing::Postprocessor, 490);

// ============================================================================
// CONTROL (500-599)
// ============================================================================

declare_lockable_resource!(Box<dyn impact_controller::MotionController>, 500);
declare_lockable_resource!(Box<dyn impact_controller::OrientationController>, 510);

// ============================================================================
// UTILITIES (600+)
// ============================================================================

declare_lockable_resource!(impact_gizmo::GizmoManager, 600);
declare_lockable_resource!(crate::instrumentation::EngineMetrics, 610);
