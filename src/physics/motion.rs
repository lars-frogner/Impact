//! Representation and computation of motion.

mod components;
mod systems;

pub use components::{AngularVelocityComp, OrientationComp, PositionComp, VelocityComp};
pub use systems::{AdvanceOrientations, AdvancePositions};

use super::fph;
use crate::geometry::{Angle, Degrees};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Point3, Quaternion, Unit, UnitQuaternion, Vector3};

/// A unit vector in 3D space.
pub type Direction = Unit<Vector3<fph>>;

/// A position in 3D space.
pub type Position = Point3<fph>;

/// A velocity in 3D space.
pub type Velocity = Vector3<fph>;

/// An orientation in 3D space.
pub type Orientation = UnitQuaternion<fph>;

/// An angular velocity in 3D space, represented by
/// an axis of rotation and an angular speed.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct AngularVelocity {
    axis_of_rotation: Direction,
    angular_speed: Degrees<fph>,
}

impl AngularVelocity {
    /// Creates a new [`AngularVelocity`] with the given
    /// axis of rotation and angular speed.
    pub fn new<A: Angle<fph>>(axis_of_rotation: Direction, angular_speed: A) -> Self {
        Self {
            axis_of_rotation,
            angular_speed: angular_speed.as_degrees(),
        }
    }

    /// Returns the axis of rotation.
    pub fn axis_of_rotation(&self) -> &Direction {
        &self.axis_of_rotation
    }

    /// Returns the angular speed.
    pub fn angular_speed(&self) -> Degrees<fph> {
        self.angular_speed
    }
}

/// Evolves the given [`Orientation`] with the given
/// [`AngularVelocity`] for the given duration.
pub fn advance_orientation(
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
    duration: fph,
) -> Orientation {
    let angle = angular_velocity.angular_speed().radians() * duration;
    let rotation = Quaternion::from_parts(
        fph::cos(0.5 * angle),
        angular_velocity
            .axis_of_rotation()
            .scale(fph::sin(0.5 * angle)),
    );
    UnitQuaternion::new_normalize(rotation * orientation.into_inner())
}
