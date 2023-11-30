//! [`Component`](impact_ecs::component::Component)s related to rigid bodies.

use crate::physics::{fph, RigidBodyID};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// rigid body with a uniform mass density.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformRigidBodyComp {
    // The mass density of the rigid body.
    pub mass_density: fph,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// [`RigidBody`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RigidBodyComp {
    /// The ID of the entity's rigid body.
    pub id: RigidBodyID,
}
