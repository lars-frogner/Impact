//! [`Component`](impact_ecs::component::Component)s related to motion.

use crate::physics::{fph, AngularVelocity, Orientation, Position, Velocity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::Vector3;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spatial position and orientation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct SpatialConfigurationComp {
    /// The offset, expressed in the entity's co-rotating reference frame, from
    /// the original origin of the entity's reference frame to the point that
    /// should be used as the actual origin.
    pub origin_offset: Vector3<fph>,
    /// The coordinates of the origin of the entity's reference frame measured
    /// in the parent space.
    pub position: Position,
    /// The 3D orientation of the entity's reference frame in the parent space.
    pub orientation: Orientation,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// physical velocity. Transparently wraps a [`Velocity`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct VelocityComp(pub Velocity);

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

impl SpatialConfigurationComp {
    /// Creates a new spatial component with the given position and orientation,
    /// retaining the original origin of the entity's reference frame.
    pub fn new(position: Position, orientation: Orientation) -> Self {
        Self::with_offset_origin(Vector3::zeros(), position, orientation)
    }

    /// Creates a new spatial component with the given position, retaining the
    /// original origin of the entity's reference frame and the identity
    /// orientation.
    pub fn unoriented(position: Position) -> Self {
        Self::new(position, Orientation::identity())
    }

    /// Creates a new spatial component with the given origin offset and
    /// position, and with the identity orientation.
    pub fn unoriented_with_offset_origin(origin_offset: Vector3<fph>, position: Position) -> Self {
        Self::with_offset_origin(origin_offset, position, Orientation::identity())
    }

    /// Creates a new spatial component with the given origin offset, position
    /// and orientation.
    pub fn with_offset_origin(
        origin_offset: Vector3<fph>,
        position: Position,
        orientation: Orientation,
    ) -> Self {
        Self {
            origin_offset,
            position,
            orientation,
        }
    }
}
