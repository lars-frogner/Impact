//! [`Component`](impact_ecs::component::Component)s related to motion.

use super::{AngularVelocity, Orientation, Position, Velocity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a spatial position. Transparently wraps a
/// [`Point3`](nalgebra::Point3) representing the 3D position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PositionComp(pub Position);

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a physical velocity. Transparently wraps a
/// [`Vector3`](nalgebra::Vector3) representing the 3D velocity.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VelocityComp(pub Velocity);

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a spatial orientation. Transparently wraps a
/// [`UnitQuaternion`](nalgebra::UnitQuaternion) representing the
/// 3D orientation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OrientationComp(pub Orientation);

/// [`Component`](impact_ecs::component::Component) for entities
/// that have an angular velocity. Transparently wraps an
/// [`AngularVelocity`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AngularVelocityComp(pub AngularVelocity);
