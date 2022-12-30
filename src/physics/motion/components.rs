//! [`Component`](impact_ecs::component::Component)s related to motion.

use super::{AngularVelocity, Orientation, Position, Velocity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a spatial position. Transparently wraps a
/// [`Point3`](nalgebra::Point3) representing the 3D position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PositionComp {
    /// A point representing 3D position.
    pub position: Position,
}

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a physical velocity. Transparently wraps a
/// [`Vector3`](nalgebra::Vector3) representing the 3D velocity.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VelocityComp {
    /// A vector representing 3D velocity.
    pub velocity: Velocity,
}

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a spatial orientation. Transparently wraps a
/// [`UnitQuaternion`](nalgebra::UnitQuaternion) representing the
/// 3D orientation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OrientationComp {
    /// A normalized quaternion representing 3D orientation.
    pub orientation: Orientation,
}

/// [`Component`](impact_ecs::component::Component) for entities
/// that have an angular velocity. Transparently wraps an
/// [`AngularVelocity`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AngularVelocityComp {
    pub angular_velocity: AngularVelocity,
}

impl PositionComp {
    /// Creates a new component representing the given position.
    pub fn new(position: Position) -> Self {
        Self { position }
    }
}

impl VelocityComp {
    /// Creates a new component representing the given velocity.
    pub fn new(velocity: Velocity) -> Self {
        Self { velocity }
    }
}

impl OrientationComp {
    /// Creates a new component representing the given orientation.
    pub fn new(orientation: Orientation) -> Self {
        Self { orientation }
    }
}

impl AngularVelocityComp {
    /// Creates a new component representing the given angular velocity.
    pub fn new(angular_velocity: AngularVelocity) -> Self {
        Self { angular_velocity }
    }
}
