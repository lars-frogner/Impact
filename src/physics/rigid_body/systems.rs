//! ECS systems related to rigid body physics.

use crate::physics::{
    motion::components::{ReferenceFrameComp, Static, VelocityComp},
    rigid_body::{components::RigidBodyComp, schemes::SchemeSubstep},
};
use impact_ecs::{query, world::World as ECSWorld};

/// Advances the motion of all applicable entities with a [`RigidBodyComp`].
pub fn advance_rigid_body_motion<S: SchemeSubstep>(ecs_world: &ECSWorld, scheme_substep: &S) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp,
         frame: &mut ReferenceFrameComp,
         velocity: &mut VelocityComp| {
            rigid_body.0.advance_motion(
                scheme_substep,
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
