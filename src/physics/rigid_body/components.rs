//! [`Component`](impact_ecs::component::Component)s related to rigid bodies.

use crate::{
    components::ComponentRegistry,
    physics::{fph, RigidBody},
};
use anyhow::Result;
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
/// rigid body. Transparently wraps a [`RigidBody`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RigidBodyComp(pub RigidBody);

/// Registers all rigid body [`Component`](impact_ecs::component::Component)s.
pub fn register_rigid_body_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, UniformRigidBodyComp)?;
    register_component!(registry, RigidBodyComp)
}
