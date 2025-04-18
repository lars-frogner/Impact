//! [`Component`](impact_ecs::component::Component)s related to rigid bodies.

use super::RigidBody;
use crate::physics::fph;
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent};
use roc_codegen::roc;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a rigid body with a uniform mass density.
///
/// The purpose of this component is to aid in constructing a [`RigidBodyComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct UniformRigidBodyComp {
    // The mass density of the rigid body.
    pub mass_density: fph,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// rigid body. Transparently wraps a [`RigidBody`].
#[roc]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RigidBodyComp(pub RigidBody);
