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

/// Advances the motion of all applicable entities with a [`RigidBodyComp`].
pub fn advance_rigid_body_motion(ecs_world: &ECSWorld, step_duration: fph) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp,
         frame: &mut ReferenceFrameComp,
         velocity: &mut VelocityComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            rigid_body.0.advance_motion(
                step_duration,
                &mut frame.position,
                &mut frame.orientation,
                frame.scaling,
                &mut velocity.linear,
                &mut velocity.angular,
            );
        },
        ![Static]
    );
}
