//! [`Component`](impact_ecs::component::Component)s related to motion.

use super::{Position, Velocity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a spatial position. Transparently wraps a [`Point3`]
/// representing the 3D position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PositionComp {
    /// A point representing 3D position.
    pub position: Position,
}

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a physical velocity. Transparently wraps a [`Vector3`]
/// representing the 3D velocity.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VelocityComp {
    /// A vector representing 3D velocity.
    pub velocity: Velocity,
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
