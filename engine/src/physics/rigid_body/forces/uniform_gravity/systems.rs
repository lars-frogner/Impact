//! ECS systems related to uniform gravity forces.

use crate::physics::{
    motion::components::Static,
    rigid_body::{
        components::RigidBodyComp, forces::uniform_gravity::components::UniformGravityComp,
    },
};
use impact_ecs::{query, world::World as ECSWorld};
use impact_scene::SceneEntityFlags;

/// Applies the force corresponding to uniform gravitational acceleration to all
/// applicable entities with a [`UniformGravityComp`].
pub fn apply_uniform_gravity(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp, gravity: &UniformGravityComp, flags: &SceneEntityFlags| {
            if flags.is_disabled() {
                return;
            }
            let force = gravity.acceleration * rigid_body.0.mass();
            rigid_body.0.apply_force_at_center_of_mass(&force);
        },
        ![Static]
    );
}
