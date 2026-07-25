//! Cleanup of anchors for removed entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator};
use impact_ecs::world::EntityEntry;
use impact_id::EntityID;
use impact_physics::rigid_body::{
    DynamicRigidBodyID, HasDynamicRigidBody, HasKinematicRigidBody, KinematicRigidBodyID,
};
use parking_lot::RwLock;

pub fn remove_anchors_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<HasDynamicRigidBody>() {
        let simulator = simulator.oread();
        let mut anchor_manager = simulator.anchor_manager().owrite();

        let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
        let anchor_ids = anchor_manager
            .dynamic_mut()
            .remove_all_anchors_for_body(rigid_body_id);

        if let Some(anchor_ids) = anchor_ids {
            let mut force_generator_manager = simulator.force_generator_manager().owrite();
            let local_force_registry = force_generator_manager.local_forces_mut();

            for anchor_id in anchor_ids {
                local_force_registry.remove_generator(anchor_id);
            }
        }
    }
    if entity.has_component::<HasKinematicRigidBody>() {
        let simulator = simulator.oread();
        let mut anchor_manager = simulator.anchor_manager().owrite();
        let rigid_body_id = KinematicRigidBodyID::from_entity_id(entity_id);
        anchor_manager
            .kinematic_mut()
            .remove_all_anchors_for_body(rigid_body_id);
    }
}
