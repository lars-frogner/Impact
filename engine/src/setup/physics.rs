//! Setup and cleanup of physics for new and removed entities.

pub mod collision;
pub mod driven_motion;
pub mod force;
pub mod rigid_body;

use crate::physics::PhysicsSimulator;
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};
use impact_mesh::MeshRepository;
use parking_lot::RwLock;

/// Performs any modifications to the physics simulator required to accommodate
/// a new entities with the given components, and adds any additional components
/// to the entities' components.
pub fn setup_physics_for_new_entities(
    simulator: &PhysicsSimulator,
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    rigid_body::setup_rigid_bodies_for_new_entities(
        simulator.rigid_body_manager(),
        mesh_repository,
        components,
    )?;

    force::setup_forces_for_new_entities(
        simulator.force_generator_manager(),
        mesh_repository,
        components,
    )?;

    driven_motion::setup_driven_motion_for_new_entities(
        simulator.motion_driver_manager(),
        components,
    );

    collision::setup_collidables_for_new_entities(simulator.collision_world(), components);

    Ok(())
}

/// Performs any modifications required to clean up the physics simulator
/// when the given entity is removed.
pub fn cleanup_physics_for_removed_entity(simulator: &PhysicsSimulator, entity: &EntityEntry<'_>) {
    impact_physics::collision::setup::remove_collidable_for_entity(
        simulator.collision_world(),
        entity,
    );

    impact_physics::driven_motion::setup::remove_motion_drivers_for_entity(
        simulator.motion_driver_manager(),
        entity,
    );

    impact_physics::force::setup::remove_force_generators_for_entity(
        simulator.force_generator_manager(),
        entity,
    );

    impact_physics::rigid_body::setup::remove_rigid_body_for_entity(
        simulator.rigid_body_manager(),
        entity,
    );
}
