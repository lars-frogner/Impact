//! ECS systems for physics.

use crate::{
    quantities::Motion,
    rigid_body::{
        DynamicRigidBodyID, HasDynamicRigidBody, HasKinematicRigidBody, KinematicRigidBodyID,
        RigidBodyManager,
    },
};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ReferenceFrame;
use impact_id::EntityID;

/// Updates the [`ReferenceFrame`] and [`Motion`] components of entities with
/// the [`DynamicRigidBodyID`] or [`KinematicRigidBodyID`] component to match
/// the current state of the rigid body.
pub fn synchronize_rigid_body_components(
    ecs_world: &ECSWorld,
    rigid_body_manager: &RigidBodyManager,
) {
    query!(
        ecs_world,
        |entity_id: EntityID, frame: &mut ReferenceFrame, motion: &mut Motion| {
            let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
            let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(rigid_body_id) else {
                return;
            };
            let linear_velocity = rigid_body.compute_velocity();
            let angular_velocity = rigid_body.compute_angular_velocity();

            frame.position = *rigid_body.position();
            frame.orientation = *rigid_body.orientation();
            motion.linear_velocity = linear_velocity.compact();
            motion.angular_velocity = angular_velocity.compact();
        },
        [HasDynamicRigidBody]
    );

    query!(
        ecs_world,
        |entity_id: EntityID, frame: &mut ReferenceFrame, motion: &mut Motion| {
            let rigid_body_id = KinematicRigidBodyID::from_entity_id(entity_id);
            let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body(rigid_body_id)
            else {
                return;
            };
            frame.position = *rigid_body.position();
            frame.orientation = *rigid_body.orientation();
            motion.linear_velocity = *rigid_body.velocity();
            motion.angular_velocity = *rigid_body.angular_velocity();
        },
        [HasKinematicRigidBody]
    );
}
