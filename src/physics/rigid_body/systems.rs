//! ECS systems related to rigid body physics.

use crate::{
    physics::{
        fph,
        motion::components::{ReferenceFrameComp, Static, VelocityComp},
        rigid_body::components::RigidBodyComp,
    },
    scene::components::SceneEntityFlagsComp,
};
use impact_ecs::{query, world::World as ECSWorld};

/// Advances the linear and angular velocities of all applicable entities with
/// a [`RigidBodyComp`].
pub fn advance_rigid_body_velocities(ecs_world: &ECSWorld, step_duration: fph) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp,
         frame: &ReferenceFrameComp,
         velocity: &mut VelocityComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            let rigid_body = &mut rigid_body.0;

            // Update the current position and orientation in case they have
            // been changed after the previous simulation step. To avoid
            // unnecessary calculations, we assume that momentum and angular
            // momentum have already been updated through the appropriate
            // methods.
            rigid_body.update_position(frame.position);
            rigid_body.update_orientation(frame.orientation);

            rigid_body.advance_momenta(step_duration);

            velocity.linear = rigid_body.compute_velocity();
            velocity.angular = rigid_body.compute_angular_velocity(frame.scaling);
        },
        ![Static]
    );
}

/// Advances the position and orientation of all applicable entities with a
/// [`RigidBodyComp`].
pub fn advance_rigid_body_configurations(ecs_world: &ECSWorld, step_duration: fph) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp,
         frame: &mut ReferenceFrameComp,
         velocity: &VelocityComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            let rigid_body = &mut rigid_body.0;

            rigid_body.advance_configuration_with(
                step_duration,
                &velocity.linear,
                &velocity.angular,
            );

            frame.position = *rigid_body.position();
            frame.orientation = *rigid_body.orientation();
        },
        ![Static]
    );
}
