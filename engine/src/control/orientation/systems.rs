//! ECS systems related to orientation control.

use crate::control::{OrientationController, orientation::components::OrientationControlComp};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ReferenceFrame;
use impact_physics::{
    fph,
    quantities::{AngularVelocity, Motion},
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager},
};

/// Updates the angular velocities of all entities controlled by the given
/// orientation controller.
pub fn update_controlled_entity_angular_velocities(
    ecs_world: &ECSWorld,
    rigid_body_manager: &mut RigidBodyManager,
    orientation_controller: &mut (impl OrientationController + ?Sized),
    time_step_duration: fph,
) {
    query!(
        ecs_world,
        |orientation_control: &mut OrientationControlComp,
         frame: &ReferenceFrame,
         motion: &mut Motion,
         rigid_body_id: &KinematicRigidBodyID| {
            let new_control_angular_velocity = if orientation_controller.orientation_has_changed() {
                let mut new_orientation = frame.orientation;
                orientation_controller.update_orientation(&mut new_orientation);

                AngularVelocity::from_consecutive_orientations(
                    &frame.orientation,
                    &new_orientation,
                    time_step_duration,
                )
            } else {
                AngularVelocity::zero()
            };

            orientation_control.apply_new_control_angular_velocity(
                new_control_angular_velocity,
                &mut motion.angular_velocity,
            );

            if let Some(rigid_body) =
                rigid_body_manager.get_kinematic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.set_angular_velocity(motion.angular_velocity);
            }
        }
    );

    query!(
        ecs_world,
        |orientation_control: &mut OrientationControlComp,
         frame: &ReferenceFrame,
         motion: &mut Motion,
         rigid_body_id: &DynamicRigidBodyID| {
            let new_control_angular_velocity = if orientation_controller.orientation_has_changed() {
                let mut new_orientation = frame.orientation;
                orientation_controller.update_orientation(&mut new_orientation);

                AngularVelocity::from_consecutive_orientations(
                    &frame.orientation,
                    &new_orientation,
                    time_step_duration,
                )
            } else {
                AngularVelocity::zero()
            };

            orientation_control.apply_new_control_angular_velocity(
                new_control_angular_velocity,
                &mut motion.angular_velocity,
            );

            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.synchronize_angular_momentum(&motion.angular_velocity);
            }
        }
    );

    orientation_controller.reset_orientation_change();
}
