//! Setup and cleanup of physics for new and removed entities.

pub mod anchor;
pub mod collision;
pub mod driven_motion;
pub mod force;
pub mod rigid_body;

use crate::{physics::PhysicsSimulator, resource::ResourceManager};
use anyhow::Result;
use impact_ecs::world::{EntityEntry, PrototypeEntities};
use impact_id::EntityID;
use parking_lot::RwLock;

/// Performs any modifications to the physics simulator required to accommodate
/// given new entities, and adds any additional components to the entities'
/// components.
pub fn setup_physics_for_new_entities(
    resource_manager: &RwLock<ResourceManager>,
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    rigid_body::setup_rigid_bodies_for_new_entities(resource_manager, simulator, entities)?;

    force::setup_forces_for_new_entities(resource_manager, simulator, entities)?;

    driven_motion::setup_driven_motion_for_new_entities(simulator, entities);

    collision::setup_collidables_for_new_entities(simulator, entities)?;

    Ok(())
}

/// Performs any modifications required to clean up the physics simulator
/// when the given entity is removed.
pub fn cleanup_physics_for_removed_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    collision::remove_collidable_for_entity(simulator, entity_id, entity);

    driven_motion::remove_motion_drivers_for_entity(simulator, entity);

    force::remove_force_generators_for_entity(simulator, entity);

    anchor::remove_anchors_for_entity(simulator, entity);

    rigid_body::remove_rigid_body_for_entity(simulator, entity);
}
