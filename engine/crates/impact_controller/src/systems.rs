//! ECS systems related to motion and orientation control.

use crate::{
    MotionController, OrientationController, motion::ControlledVelocity,
    orientation::ControlledAngularVelocity,
};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ReferenceFrame;
use impact_physics::{
    quantities::{AngularVelocity, Motion},
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
        |controlled_velocity: &mut ControlledVelocity,
         motion: &mut Motion,
         frame: &ReferenceFrame,
         rigid_body_id: &KinematicRigidBodyID| {
            let orientation = frame.orientation.unpack();

            let new_controlled_velocity =
                motion_controller.compute_controlled_velocity(&orientation);

            let mut linear_velocity = motion.linear_velocity.unpack();

            controlled_velocity
                .apply_new_controlled_velocity(new_controlled_velocity, &mut linear_velocity);

            motion.linear_velocity = linear_velocity.pack();

            if let Some(rigid_body) =
                rigid_body_manager.get_kinematic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.set_velocity(motion.linear_velocity);
            }
        }
    );

    query!(
        ecs_world,
        |controlled_velocity: &mut ControlledVelocity,
         motion: &mut Motion,
         frame: &ReferenceFrame,
         rigid_body_id: &DynamicRigidBodyID| {
            let orientation = frame.orientation.unpack();

            let new_controlled_velocity =
                motion_controller.compute_controlled_velocity(&orientation);

            let mut linear_velocity = motion.linear_velocity.unpack();

            controlled_velocity
                .apply_new_controlled_velocity(new_controlled_velocity, &mut linear_velocity);

            motion.linear_velocity = linear_velocity.pack();

            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.synchronize_momentum(&linear_velocity);
            }
        }
    );
}

/// Updates the angular velocities of all entities controlled by the given
/// orientation controller.
pub fn update_controlled_entity_angular_velocities(
    ecs_world: &ECSWorld,
    rigid_body_manager: &mut RigidBodyManager,
    orientation_controller: &mut (impl OrientationController + ?Sized),
    time_step_duration: f32,
) {
    query!(
        ecs_world,
        |controlled_angular_velocity: &mut ControlledAngularVelocity,
         frame: &ReferenceFrame,
         motion: &mut Motion,
         rigid_body_id: &KinematicRigidBodyID| {
            let new_controlled_angular_velocity =
                if orientation_controller.orientation_has_changed() {
                    let old_orientation = frame.orientation.unpack();
                    let mut new_orientation = old_orientation;

                    orientation_controller.update_orientation(&mut new_orientation);

                    AngularVelocity::from_consecutive_orientations(
                        &old_orientation,
                        &new_orientation,
                        time_step_duration,
                    )
                } else {
                    AngularVelocity::zero()
                };

            let mut angular_velocity = motion.angular_velocity.unpack();

            controlled_angular_velocity.apply_new_controlled_angular_velocity(
                new_controlled_angular_velocity,
                &mut angular_velocity,
            );

            motion.angular_velocity = angular_velocity.pack();

            if let Some(rigid_body) =
                rigid_body_manager.get_kinematic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.set_angular_velocity(motion.angular_velocity);
            }
        }
    );

    query!(
        ecs_world,
        |controlled_angular_velocity: &mut ControlledAngularVelocity,
         frame: &ReferenceFrame,
         motion: &mut Motion,
         rigid_body_id: &DynamicRigidBodyID| {
            let new_controlled_angular_velocity =
                if orientation_controller.orientation_has_changed() {
                    let old_orientation = frame.orientation.unpack();
                    let mut new_orientation = old_orientation;

                    orientation_controller.update_orientation(&mut new_orientation);

                    AngularVelocity::from_consecutive_orientations(
                        &old_orientation,
                        &new_orientation,
                        time_step_duration,
                    )
                } else {
                    AngularVelocity::zero()
                };

            let mut angular_velocity = motion.angular_velocity.unpack();

            controlled_angular_velocity.apply_new_controlled_angular_velocity(
                new_controlled_angular_velocity,
                &mut angular_velocity,
            );

            motion.angular_velocity = angular_velocity.pack();

            if let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body_mut(*rigid_body_id)
            {
                rigid_body.synchronize_angular_momentum(&angular_velocity);
            }
        }
    );

    orientation_controller.reset_orientation_change();
}
