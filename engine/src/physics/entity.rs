//! Management of physics for entities.

use crate::physics::{PhysicsSimulator, collision, driven_motion, force, rigid_body};
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};
use impact_mesh::MeshRepository;
use std::sync::RwLock;

/// Performs any modifications to the physics simulator required to
/// accommodate a new entity with the given components, and adds any
/// additional components to the entity's components.
pub fn setup_physics_for_new_entity(
    simulator: &PhysicsSimulator,
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    rigid_body::entity::setup_rigid_body_for_new_entity(
        simulator.rigid_body_manager(),
        mesh_repository,
        components,
    )?;

    force::entity::setup_forces_for_new_entity(
        simulator.force_generator_manager(),
        mesh_repository,
        components,
    )?;

    driven_motion::entity::setup_driven_motion_for_new_entity(
        simulator.motion_driver_manager(),
        components,
    );

    collision::entity::setup_collidable_for_new_entity(simulator.collision_world(), components);

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
