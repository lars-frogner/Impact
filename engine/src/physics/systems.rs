//! ECS systems for physics.

use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ReferenceFrame;
use impact_physics::{
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager},
};

/// Updates the [`ReferenceFrame`] and [`Motion`] components of entities with
/// the [`DynamicRigidBodyID`] or [`KinematicRigidBodyID`] component to match
/// the current state of the rigid body.
pub fn synchronize_rigid_body_components(
    ecs_world: &ECSWorld,
    rigid_body_manager: &RigidBodyManager,
) {
    query!(
        ecs_world,
        |frame: &mut ReferenceFrame, motion: &mut Motion, rigid_body_id: &DynamicRigidBodyID| {
            let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(*rigid_body_id) else {
                return;
            };
            frame.position = *rigid_body.position();
            frame.orientation = *rigid_body.orientation();
            motion.linear_velocity = rigid_body.compute_velocity();
            motion.angular_velocity = rigid_body.compute_angular_velocity();
        }
    );

    query!(
        ecs_world,
        |frame: &mut ReferenceFrame, motion: &mut Motion, rigid_body_id: &KinematicRigidBodyID| {
            let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body(*rigid_body_id)
            else {
                return;
            };
            frame.position = *rigid_body.position();
            frame.orientation = *rigid_body.orientation();
            motion.linear_velocity = *rigid_body.velocity();
            motion.angular_velocity = *rigid_body.angular_velocity();
        }
    );
}
