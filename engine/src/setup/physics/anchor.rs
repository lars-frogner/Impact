//! Cleanup of anchors for removed entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator};
use impact_ecs::world::EntityEntry;
use impact_physics::rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID};
use parking_lot::RwLock;

pub fn remove_anchors_for_entity(simulator: &RwLock<PhysicsSimulator>, entity: &EntityEntry<'_>) {
    if let Some(rigid_body_id) = entity.get_component::<DynamicRigidBodyID>() {
        let simulator = simulator.oread();
        let mut anchor_manager = simulator.anchor_manager().owrite();
        anchor_manager
            .dynamic_mut()
            .remove_all_anchors_for_body(*rigid_body_id.access());
    }
    if let Some(rigid_body_id) = entity.get_component::<KinematicRigidBodyID>() {
        let simulator = simulator.oread();
        let mut anchor_manager = simulator.anchor_manager().owrite();
        anchor_manager
            .kinematic_mut()
            .remove_all_anchors_for_body(*rigid_body_id.access());
    }
}
