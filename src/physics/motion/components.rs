//! [`Component`](impact_ecs::component::Component)s related to motion.

use crate::physics::{fph, AngularVelocity, Orientation, Position, Velocity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spatial position. Transparently wraps a [`Point3`](nalgebra::Point3)
/// representing the 3D position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PositionComp(pub Position);

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// physical velocity. Transparently wraps a [`Vector3`](nalgebra::Vector3)
/// representing the 3D velocity.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VelocityComp(pub Velocity);

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// spatial orientation. Transparently wraps a
/// [`UnitQuaternion`](nalgebra::UnitQuaternion) representing the 3D
/// orientation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OrientationComp(pub Orientation);

/// [`Component`](impact_ecs::component::Component) for entities that have an
/// angular velocity about their center of mass. Transparently wraps an
/// [`AngularVelocity`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AngularVelocityComp(pub AngularVelocity);

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// driven angular velocity about an arbitrary center of rotation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DrivenAngularVelocityComp {
    /// The angular velocity.
    pub angular_velocity: AngularVelocity,
    /// The center of rotation, defined in the model's reference frame.
    pub center_of_rotation: Position,
}

/// Marker [`Component`](impact_ecs::component::Component) for entities whose
/// position and orientation are not supposed to change.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct Static;

impl DrivenAngularVelocityComp {
    /// Creates a new component representing driven angular velocity about the
    /// given center of rotation, which is defined in the model's reference
    /// frame.
    pub fn new(angular_velocity: AngularVelocity, center_of_rotation: Position) -> Self {
        Self {
            angular_velocity,
            center_of_rotation,
        }
    }

    /// Creates a new component representing driven angular velocity about the
    /// model space origin.
    pub fn new_about_model_origin(angular_velocity: AngularVelocity) -> Self {
        Self::new(angular_velocity, Position::origin())
    }

    /// Evolves the given model orientation with the angular velocity for the
    /// given duration and shifts the given position, assumed to be the world
    /// space coordinates of the model space origin, to account for the offset
    /// center of rotation.
    pub fn advance_orientation_and_shift_reference_frame(
        &self,
        orientation: &mut Orientation,
        reference_frame_origin: &mut Position,
        duration: fph,
    ) {
        let new_orientation =
            super::advance_orientation(orientation, &self.angular_velocity, duration);

        // The position, which is the world space coordinates of the model space
        // origin, is by default unaffected by the rotation. This is only
        // correct if the center of rotation is at the model space origin. If
        // the center of rotation is somewhere else, the model's reference frame
        // must be displaced in world space so that the center of rotation does
        // not move.
        *reference_frame_origin += super::compute_model_origin_shift_from_orientation_change(
            orientation,
            &new_orientation,
            &self.center_of_rotation,
        );

        *orientation = new_orientation;
    }
}
