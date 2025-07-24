//! `LockableResource` implementations for Engine types.

use crate::declare_lockable_resource;

// ============================================================================
// CORE ECS AND ENTITY MANAGEMENT (0-99)
// ============================================================================

declare_lockable_resource!(crate::component::ComponentRegistry, 10);

// EntityStager is often acquired before ECSWorld in tasks like ApplyVoxelAbsorption
declare_lockable_resource!(impact_ecs::world::EntityStager, 20);

// ECSWorld is World from impact_ecs crate (aliased as ECSWorld in engine.rs)
declare_lockable_resource!(impact_ecs::world::World, 30);

// ============================================================================
// SCENE AND ASSET MANAGEMENT (100-199)
// ============================================================================

// Scene is frequently accessed early in task chains and contains many sub-resources
declare_lockable_resource!(crate::scene::Scene, 100);

// Assets are typically accessed after scene for loading resources
declare_lockable_resource!(impact_assets::Assets, 110);

// ============================================================================
// RENDERING SYSTEMS (200-299)
// ============================================================================

// RenderingSystem often accessed after scene for rendering operations
declare_lockable_resource!(crate::rendering::RenderingSystem, 200);

// ============================================================================
// PHYSICS SIMULATION (300-399)
// ============================================================================

// PhysicsSimulator is a type alias to impact_physics::PhysicsSimulator
// Often accessed after scene for physics operations
declare_lockable_resource!(
    impact_physics::PhysicsSimulator<impact_voxel::collidable::Collidable>,
    300
);

// ============================================================================
// INPUT AND CONTROLLERS (400-499)
// ============================================================================

// Controllers - these are trait objects so we implement on the boxed types
// Typically accessed independently and less frequently in task chains
declare_lockable_resource!(Box<dyn impact_controller::MotionController>, 400);

declare_lockable_resource!(Box<dyn impact_controller::OrientationController>, 410);

// ============================================================================
// AUXILIARY SYSTEMS (500+)
// ============================================================================

// GizmoManager is often accessed after core systems for debug visualization
declare_lockable_resource!(crate::gizmo::GizmoManager, 500);

// EngineMetrics for performance tracking, typically accessed independently
declare_lockable_resource!(crate::instrumentation::EngineMetrics, 510);

// GameLoopController for controlling execution flow
declare_lockable_resource!(crate::game_loop::GameLoopController, 520);
