//! [`Component`](impact_ecs::component::Component)s related to motion.

use super::fmo;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{Point3, Vector3};

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a spatial position. Transparently wraps a [`Point3`]
/// representing the 3D position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PositionComp {
    /// A point representing 3D position.
    pub point: Point3<fmo>,
}

/// [`Component`](impact_ecs::component::Component) for entities
/// that have a physical velocity. Transparently wraps a [`Vector3`]
/// representing the 3D velocity.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VelocityComp {
    /// A vector representing 3D velocity.
    pub vector: Vector3<fmo>,
}
