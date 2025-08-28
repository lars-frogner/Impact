//! `LockableResource` implementations for Engine types.

use crate::declare_lockable_resource;

// ============================================================================
// GAME LOOP (0-19)
// ============================================================================

declare_lockable_resource!(crate::game_loop::GameLoopController, 10);

// ============================================================================
// ECS (20-49)
// ============================================================================

declare_lockable_resource!(impact_ecs::world::EntityStager, 20);
declare_lockable_resource!(impact_ecs::world::World, 30);

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
declare_lockable_resource!(impact_scene::camera::CameraManager, 220);
declare_lockable_resource!(impact_light::LightManager, 230);
declare_lockable_resource!(impact_voxel::VoxelObjectManager, 240);
declare_lockable_resource!(impact_scene::model::ModelInstanceManager, 250);
declare_lockable_resource!(impact_scene::graph::SceneGraph, 260);

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
// RENDERING (400-499)
// ============================================================================

declare_lockable_resource!(crate::rendering::RenderingSystem, 400);

// RenderingSystem components (must be acquired after RenderingSystem itself)
// Order follows rendering pipeline flow and dependency hierarchy:
// Resources → Shaders → Render Targets → Resource Groups → Storage → Post-processing
declare_lockable_resource!(crate::rendering::resource::RenderResourceManager, 410);
declare_lockable_resource!(impact_gpu::shader::ShaderManager, 420);
declare_lockable_resource!(
    impact_rendering::attachment::RenderAttachmentTextureManager,
    430
);
declare_lockable_resource!(impact_gpu::resource_group::GPUResourceGroupManager, 440);
declare_lockable_resource!(impact_gpu::storage::StorageGPUBufferManager, 450);
declare_lockable_resource!(impact_rendering::postprocessing::Postprocessor, 460);

// ============================================================================
// CONTROL (500-599)
// ============================================================================

declare_lockable_resource!(Box<dyn impact_controller::MotionController>, 500);
declare_lockable_resource!(Box<dyn impact_controller::OrientationController>, 510);

// ============================================================================
// UTILITIES (600+)
// ============================================================================

declare_lockable_resource!(crate::gizmo::GizmoManager, 600);
declare_lockable_resource!(crate::instrumentation::EngineMetrics, 610);
