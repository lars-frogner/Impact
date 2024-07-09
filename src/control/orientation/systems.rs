//! ECS systems related to orientation control.

use crate::{
    control::{orientation::components::OrientationControlComp, OrientationController},
    physics::{
        fph,
        motion::{
            components::{ReferenceFrameComp, VelocityComp},
            AngularVelocity,
        },
        rigid_body::components::RigidBodyComp,
    },
};
use impact_ecs::{query, world::World as ECSWorld};

/// Updates the angular velocities and/or orientations of all entities
/// controlled by the given orientation controller.
pub fn update_rotation_of_controlled_entities(
    ecs_world: &ECSWorld,
    orientation_controller: &mut (impl OrientationController + ?Sized),
    time_step_duration: fph,
) {
    if orientation_controller.orientation_has_changed() {
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp| {
                orientation_controller.update_orientation(&mut frame.orientation);
            },
            [OrientationControlComp],
            ![VelocityComp, RigidBodyComp]
        );
    }
    query!(
        ecs_world,
        |orientation_control: &mut OrientationControlComp,
         frame: &mut ReferenceFrameComp,
         velocity: &mut VelocityComp| {
            let new_control_angular_velocity = if orientation_controller.orientation_has_changed() {
                let old_orientation = frame.orientation;
                orientation_controller.update_orientation(&mut frame.orientation);

                AngularVelocity::from_consecutive_orientations(
                    &old_orientation,
                    &frame.orientation,
                    time_step_duration,
                )
            } else {
                AngularVelocity::zero()
            };

            orientation_control.apply_new_control_angular_velocity(
                new_control_angular_velocity,
                &mut velocity.angular,
            );
        },
        ![RigidBodyComp]
    );
    query!(
        ecs_world,
        |orientation_control: &mut OrientationControlComp,
         rigid_body: &mut RigidBodyComp,
         frame: &ReferenceFrameComp,
         velocity: &mut VelocityComp| {
            let new_control_angular_velocity = if orientation_controller.orientation_has_changed() {
                // We do not update the orientation here, as the rigid body
                // motion system will handle that for us as long as we apply the
                // correct angular velocity
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
                &mut velocity.angular,
            );

            rigid_body.0.synchronize_angular_momentum(
                &frame.orientation,
                frame.scaling,
                &velocity.angular,
            );
        }
    );

    orientation_controller.reset_orientation_change();
}
