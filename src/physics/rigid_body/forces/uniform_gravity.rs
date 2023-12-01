//! Uniform gravitational acceleration.

use crate::physics::{fph, RigidBodyComp};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{query, world::World as ECSWorld, Component};
use nalgebra::{vector, Vector3};

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// uniform gravitational acceleration.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformGravityComp {
    /// The gravitational acceleration of the entity.
    pub acceleration: Vector3<fph>,
}

impl UniformGravityComp {
    /// Creates a new component for uniform gravitational acceleration.
    pub fn new(acceleration: Vector3<fph>) -> Self {
        Self { acceleration }
    }

    /// Creates a new component for uniform gravitational acceleration in the
    /// negative y-direction.
    pub fn downward(acceleration: fph) -> Self {
        Self::new(vector![0.0, -acceleration, 0.0])
    }
}

/// Applies the force corresponding to uniform gravitational acceleration to all
/// applicable rigid bodies.
pub fn apply_uniform_gravity(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp, gravity: &UniformGravityComp| {
            let force = gravity.acceleration * rigid_body.0.mass();
            rigid_body.0.apply_force_at_center_of_mass(&force);
        }
    );
}
