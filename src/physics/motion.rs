//! Representation and computation of motion.

mod components;
mod systems;

pub use components::{AngularVelocityComp, OrientationComp, PositionComp, Static, VelocityComp};
pub use systems::{AdvanceOrientations, AdvancePositions};

use super::fph;
use crate::geometry::{Angle, Radians};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Point3, Quaternion, SimdComplexField, Unit, UnitQuaternion, Vector3};

/// A unit vector in 3D space.
pub type Direction = Unit<Vector3<fph>>;

/// A position in 3D space.
pub type Position = Point3<fph>;

/// A velocity in 3D space.
pub type Velocity = Vector3<fph>;

/// An orientation in 3D space.
pub type Orientation = UnitQuaternion<fph>;

/// An angular velocity in 3D space, represented by an axis of rotation, a
/// center of rotation in the model's reference frame and an angular speed.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct AngularVelocity {
    axis_of_rotation: Direction,
    center_of_rotation: Position,
    angular_speed: Radians<fph>,
}

impl AngularVelocity {
    /// Creates a new [`AngularVelocity`] with the given axis of rotation,
    /// center of rotation (a point in the model's reference frame the axis of
    /// rotation goes through) and angular speed.
    pub fn new<A: Angle<fph>>(
        axis_of_rotation: Direction,
        center_of_rotation: Position,
        angular_speed: A,
    ) -> Self {
        Self {
            axis_of_rotation,
            center_of_rotation,
            angular_speed: angular_speed.as_radians(),
        }
    }

    /// Creates a new [`AngularVelocity`] with the given axis of rotation and
    /// angular speed. The axis of rotation is assumed to pass through the model
    /// space origin.
    pub fn new_about_model_origin<A: Angle<fph>>(
        axis_of_rotation: Direction,
        angular_speed: A,
    ) -> Self {
        Self::new(axis_of_rotation, Position::origin(), angular_speed)
    }

    /// Returns the axis of rotation.
    pub fn axis_of_rotation(&self) -> &Direction {
        &self.axis_of_rotation
    }

    /// Returns the center of rotation in the model's reference frame.
    pub fn center_of_rotation(&self) -> &Position {
        &self.center_of_rotation
    }

    /// Returns the angular speed.
    pub fn angular_speed(&self) -> Radians<fph> {
        self.angular_speed
    }
}

/// Evolves the given [`Orientation`] with the given [`AngularVelocity`] for the
/// given duration.
pub fn advance_orientation(
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
    duration: fph,
) -> Orientation {
    let angle = angular_velocity.angular_speed().radians() * duration;
    let (sin_half_angle, cos_half_angle) = (0.5 * angle).simd_sin_cos();

    let rotation = Quaternion::from_parts(
        cos_half_angle,
        angular_velocity.axis_of_rotation().scale(sin_half_angle),
    );

    UnitQuaternion::new_normalize(rotation * orientation.into_inner())
}

/// Computes the world space displacement of the model space origin resulting
/// from a change in model orientation around the given center of rotation,
/// which is specified in the model's reference frame.
pub fn compute_model_origin_shift_from_orientation_change(
    old_orientation: &Orientation,
    new_orientation: &Orientation,
    center_of_rotation: &Position,
) -> Vector3<fph> {
    old_orientation.transform_point(center_of_rotation)
        - new_orientation.transform_point(center_of_rotation)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geometry::{Degrees, Radians};
    use approx::assert_abs_diff_eq;

    #[test]
    fn advancing_orientation_with_zero_angular_speed_gives_same_orientation() {
        let orientation = Orientation::identity();
        let angular_velocity =
            AngularVelocity::new_about_model_origin(Vector3::x_axis(), Degrees(0.0));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, 1.2);
        assert_abs_diff_eq!(advanced_orientation, orientation);
    }

    #[test]
    fn advancing_orientation_by_zero_duration_gives_same_orientation() {
        let orientation = Orientation::identity();
        let angular_velocity =
            AngularVelocity::new_about_model_origin(Vector3::x_axis(), Degrees(1.2));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, 0.0);
        assert_abs_diff_eq!(advanced_orientation, orientation);
    }

    #[test]
    fn advancing_orientation_about_its_own_axis_works() {
        let angular_speed = 0.1;
        let duration = 2.0;
        let orientation = Orientation::from_axis_angle(&Vector3::y_axis(), 0.1);
        let angular_velocity =
            AngularVelocity::new_about_model_origin(Vector3::y_axis(), Radians(angular_speed));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, duration);
        assert_abs_diff_eq!(
            advanced_orientation.angle(),
            orientation.angle() + angular_speed * duration
        );
        assert_abs_diff_eq!(
            advanced_orientation.axis().unwrap(),
            orientation.axis().unwrap()
        );
    }
}
