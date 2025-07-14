//! ECS systems related to motion control.

use crate::control::{MotionController, motion::components::MotionControlComp};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ReferenceFrame;
use impact_physics::{
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager},
};

/// Updates the world-space velocities of all entities controlled by the given
/// motion controller.
pub fn update_controlled_entity_velocities(
    ecs_world: &ECSWorld,
    rigid_body_manager: &mut RigidBodyManager,
    motion_controller: &(impl MotionController + ?Sized),
) {
    query!(
        ecs_world,
        |motion_control: &mut MotionControlComp,
         motion: &mut Motion,
         frame: &ReferenceFrame,
         rigid_body_id: &KinematicRigidBodyID| {
            let new_control_velocity =
                motion_controller.compute_control_velocity(&frame.orientation);

            motion_control
                .apply_new_control_velocity(new_control_velocity, &mut motion.linear_velocity);

            if let Some(rigid_body) =
                rigid_body_manager.get_kinematic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.set_velocity(motion.linear_velocity);
            }
        }
    );

    query!(
        ecs_world,
        |motion_control: &mut MotionControlComp,
         motion: &mut Motion,
         frame: &ReferenceFrame,
         rigid_body_id: &DynamicRigidBodyID| {
            let new_control_velocity =
                motion_controller.compute_control_velocity(&frame.orientation);

            motion_control
                .apply_new_control_velocity(new_control_velocity, &mut motion.linear_velocity);

            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.synchronize_momentum(&motion.linear_velocity);
            }
        }
    );
}
