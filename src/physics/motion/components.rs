//! [`Component`](impact_ecs::component::Component)s related to motion.

use crate::physics::{fph, AngularVelocity, Orientation, Position, Velocity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::Vector3;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spatial position.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct PositionComp {
    /// The offset, expressed in the entity's co-rotating reference frame, from
    /// the original origin of the entity's reference frame to the point that
    /// should be used as the actual origin.
    pub origin_offset: Vector3<fph>,
    /// The coordinates of the origin of the entity's reference frame measured
    /// in the parent space.
    pub position: Position,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// physical velocity. Transparently wraps a [`Vector3`](nalgebra::Vector3)
/// representing the 3D velocity.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct VelocityComp(pub Velocity);

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spatial orientation. Transparently wraps a
/// [`UnitQuaternion`](nalgebra::UnitQuaternion) representing the 3D
/// orientation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct OrientationComp(pub Orientation);

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// angular velocity about their center of mass. Transparently wraps an
/// [`AngularVelocity`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct AngularVelocityComp(pub AngularVelocity);

/// Marker [`Component`](impact_ecs::component::Component) for entities whose
/// position and orientation are not supposed to change.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct Static;

impl PositionComp {
    /// Creates a new position component with the given origin offset and
    /// position.
    pub fn new(origin_offset: Vector3<fph>, position: Position) -> Self {
        Self {
            origin_offset,
            position,
        }
    }

    /// Creates a new position component with the given position, retaining the
    /// original origin of the entity's reference frame.
    pub fn unoffset(position: Position) -> Self {
        Self::new(Vector3::zeros(), position)
    }
}
