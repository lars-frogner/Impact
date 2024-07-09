//! Uniform gravitational acceleration.

pub mod components;

use crate::physics::{motion::components::Static, rigid_body::components::RigidBodyComp};
use components::UniformGravityComp;
use impact_ecs::{query, world::World as ECSWorld};

/// Applies the force corresponding to uniform gravitational acceleration to all
/// applicable rigid bodies.
pub fn apply_uniform_gravity(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp, gravity: &UniformGravityComp| {
            let force = gravity.acceleration * rigid_body.0.mass();
            rigid_body.0.apply_force_at_center_of_mass(&force);
        },
        ![Static]
    );
}
