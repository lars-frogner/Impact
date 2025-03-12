//! ECS systems related to rigid body physics.

use crate::{
    physics::{
        fph,
        motion::{
            self,
            components::{ReferenceFrameComp, Static, VelocityComp},
        },
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

            velocity.linear = rigid_body
                .0
                .compute_advanced_velocity(&velocity.linear, step_duration);

            rigid_body.0.advance_angular_momentum(step_duration);

            velocity.angular = rigid_body
                .0
                .compute_angular_velocity(&frame.orientation, frame.scaling);
        },
        ![Static]
    );
}

/// Advances the position and orientation of all applicable entities with a
/// [`RigidBodyComp`].
pub fn advance_rigid_body_configurations(ecs_world: &ECSWorld, step_duration: fph) {
    query!(
        ecs_world,
        |frame: &mut ReferenceFrameComp, velocity: &VelocityComp, flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }

            frame.position =
                motion::advance_position(&frame.position, &velocity.linear, step_duration);

            frame.orientation =
                motion::advance_orientation(&frame.orientation, &velocity.angular, step_duration);
        },
        [RigidBodyComp],
        ![Static]
    );
}
