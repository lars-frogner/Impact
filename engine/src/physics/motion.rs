//! Representation and computation of motion.

pub mod analytical;
pub mod components;
pub mod systems;
pub mod tasks;

use crate::{
    geometry::{Angle, Radians},
    physics::{fph, inertia::InertialProperties},
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use nalgebra::{Point3, Quaternion, Unit, UnitQuaternion, UnitVector3, Vector3};
use roc_codegen::roc;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// A unit vector in 3D space.
pub type Direction = Unit<Vector3<fph>>;

/// A position in 3D space.
pub type Position = Point3<fph>;

/// A velocity in 3D space.
pub type Velocity = Vector3<fph>;

/// An orientation in 3D space.
pub type Orientation = UnitQuaternion<fph>;

/// An angular velocity in 3D space, represented by an axis of rotation and an
/// angular speed.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct AngularVelocity {
    axis_of_rotation: Direction,
    angular_speed: Radians<fph>,
}

/// A momentum in 3D space.
pub type Momentum = Vector3<fph>;

/// An angular momentum in 3D space.
pub type AngularMomentum = Vector3<fph>;

/// An acceleration in 3D space.
pub type Acceleration = Vector3<fph>;

/// An angular acceleration in 3D space.
pub type AngularAcceleration = Vector3<fph>;

/// A 3D force.
pub type Force = Vector3<fph>;

/// A 3D torque.
pub type Torque = Vector3<fph>;

impl AngularVelocity {
    /// Creates a new [`AngularVelocity`] with the given axis of rotation and
    /// angular speed.
    pub fn new<A: Angle<fph>>(axis_of_rotation: Direction, angular_speed: A) -> Self {
        Self {
            axis_of_rotation,
            angular_speed: angular_speed.as_radians(),
        }
    }

    /// Creates a new [`AngularVelocity`] from the given angular velocity
    /// vector.
    pub fn from_vector(angular_velocity_vector: Vector3<fph>) -> Self {
        if let Some((axis_of_rotation, angular_speed)) =
            UnitVector3::try_new_and_get(angular_velocity_vector, fph::EPSILON)
        {
            Self::new(axis_of_rotation, Radians(angular_speed))
        } else {
            Self::zero()
        }
    }

    /// Creates the [`AngularVelocity`] that would change the given first
    /// orientation to the given second orientation if applied for the given
    /// duration.
    pub fn from_consecutive_orientations(
        first_orientation: &Orientation,
        second_orientation: &Orientation,
        duration: fph,
    ) -> Self {
        let difference = second_orientation * first_orientation.inverse();
        if let Some((axis, angle)) = difference.axis_angle() {
            Self::new(axis, Radians(angle / duration))
        } else {
            Self::zero()
        }
    }

    /// Creates a new [`AngularVelocity`] with zero angular speed.
    pub fn zero() -> Self {
        Self {
            axis_of_rotation: Vector3::y_axis(),
            angular_speed: Radians(0.0),
        }
    }

    /// Returns the axis of rotation.
    pub fn axis_of_rotation(&self) -> &Direction {
        &self.axis_of_rotation
    }

    /// Returns the angular speed.
    pub fn angular_speed(&self) -> Radians<fph> {
        self.angular_speed
    }

    /// Computes the corresponding angular velocity vector.
    pub fn as_vector(&self) -> Vector3<fph> {
        self.axis_of_rotation.as_ref() * self.angular_speed.radians()
    }
}

impl Add for &AngularVelocity {
    type Output = AngularVelocity;

    fn add(self, rhs: Self) -> AngularVelocity {
        AngularVelocity::from_vector(self.as_vector() + rhs.as_vector())
    }
}

impl Sub for &AngularVelocity {
    type Output = AngularVelocity;

    fn sub(self, rhs: Self) -> AngularVelocity {
        AngularVelocity::from_vector(self.as_vector() - rhs.as_vector())
    }
}

impl AddAssign for AngularVelocity {
    fn add_assign(&mut self, rhs: Self) {
        *self = AngularVelocity::from_vector(self.as_vector() + rhs.as_vector());
    }
}

impl SubAssign for AngularVelocity {
    fn sub_assign(&mut self, rhs: Self) {
        *self = AngularVelocity::from_vector(self.as_vector() - rhs.as_vector());
    }
}

impl Default for AngularVelocity {
    fn default() -> Self {
        Self::zero()
    }
}

impl AbsDiffEq for AngularVelocity {
    type Epsilon = <fph as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        fph::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Direction::abs_diff_eq(&self.axis_of_rotation, &other.axis_of_rotation, epsilon)
            && Radians::abs_diff_eq(&self.angular_speed, &other.angular_speed, epsilon)
    }
}

/// Computes the quaternion representing the instantaneous time derivative of
/// the given [`Orientation`] for a body with the given angular velocity.
pub fn compute_orientation_derivative(
    orientation: &Orientation,
    angular_velocity_vector: &Vector3<fph>,
) -> Quaternion<fph> {
    Quaternion::from_imag(0.5 * angular_velocity_vector) * orientation.as_ref()
}

/// Evolves the given [`Position`] linearly with the given [`Velocity`] for the
/// given duration.
pub fn advance_position(position: &Position, velocity: &Velocity, duration: fph) -> Position {
    position + velocity * duration
}

/// Evolves the given [`Orientation`] with the given [`AngularVelocity`] for the
/// given duration.
pub fn advance_orientation(
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
    duration: fph,
) -> Orientation {
    let angle = angular_velocity.angular_speed().radians() * duration;
    let (sin_half_angle, cos_half_angle) = (0.5 * angle).sin_cos();

    let rotation = Quaternion::from_parts(
        cos_half_angle,
        angular_velocity.axis_of_rotation().scale(sin_half_angle),
    );

    UnitQuaternion::new_normalize(rotation * orientation.as_ref())
}

/// Computes the [`AngularVelocity`] of a body with the given properties.
pub fn compute_angular_velocity(
    inertial_properties: &InertialProperties,
    orientation: &Orientation,
    scaling: fph,
    angular_momentum: &AngularMomentum,
) -> AngularVelocity {
    let inverse_world_space_inertia_tensor = inertial_properties
        .inertia_tensor()
        .inverse_rotated_matrix_with_scaled_extent(orientation, scaling);

    AngularVelocity::from_vector(inverse_world_space_inertia_tensor * angular_momentum)
}

/// Computes the [`AngularMomentum`] of a body with the given properties.
pub fn compute_angular_momentum(
    inertial_properties: &InertialProperties,
    orientation: &Orientation,
    scaling: fph,
    angular_velocity: &AngularVelocity,
) -> AngularMomentum {
    inertial_properties
        .inertia_tensor()
        .rotated_matrix_with_scaled_extent(orientation, scaling)
        * angular_velocity.as_vector()
}

/// Computes the [`AngularAcceleration`] of a body with the given properties
/// when the body experiences the given [`Torque`] around its center of mass.
pub fn compute_angular_acceleration(
    inertial_properties: &InertialProperties,
    orientation: &Orientation,
    scaling: fph,
    torque: &Torque,
) -> AngularAcceleration {
    inertial_properties
        .inertia_tensor()
        .inverse_rotated_matrix_with_scaled_extent(orientation, scaling)
        * torque
}

/// Computes the total kinetic energy (translational and rotational) of a
/// body with the given properties.
pub fn compute_total_kinetic_energy(
    inertial_properties: &InertialProperties,
    orientation: &Orientation,
    scaling: fph,
    velocity: &Velocity,
    angular_velocity: &AngularVelocity,
) -> fph {
    compute_translational_kinetic_energy(inertial_properties.mass(), velocity)
        + compute_rotational_kinetic_energy(
            inertial_properties,
            orientation,
            scaling,
            angular_velocity,
        )
}

/// Computes the translational kinetic energy of a body with the given
/// properties.
pub fn compute_translational_kinetic_energy(mass: fph, velocity: &Velocity) -> fph {
    0.5 * mass * velocity.norm_squared()
}

/// Computes the rotational kinetic energy of a body with the given properties.
pub fn compute_rotational_kinetic_energy(
    inertial_properties: &InertialProperties,
    orientation: &Orientation,
    scaling: fph,
    angular_velocity: &AngularVelocity,
) -> fph {
    let angular_momentum =
        compute_angular_momentum(inertial_properties, orientation, scaling, angular_velocity);

    0.5 * angular_velocity.as_vector().dot(&angular_momentum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Degrees, Radians};
    use approx::assert_abs_diff_eq;

    #[test]
    fn advancing_orientation_with_zero_angular_speed_gives_same_orientation() {
        let orientation = Orientation::identity();
        let angular_velocity = AngularVelocity::new(Vector3::x_axis(), Degrees(0.0));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, 1.2);
        assert_abs_diff_eq!(advanced_orientation, orientation);
    }

    #[test]
    fn advancing_orientation_by_zero_duration_gives_same_orientation() {
        let orientation = Orientation::identity();
        let angular_velocity = AngularVelocity::new(Vector3::x_axis(), Degrees(1.2));
        let advanced_orientation = advance_orientation(&orientation, &angular_velocity, 0.0);
        assert_abs_diff_eq!(advanced_orientation, orientation);
    }

    #[test]
    fn advancing_orientation_about_its_own_axis_works() {
        let angular_speed = 0.1;
        let duration = 2.0;
        let orientation = Orientation::from_axis_angle(&Vector3::y_axis(), 0.1);
        let angular_velocity = AngularVelocity::new(Vector3::y_axis(), Radians(angular_speed));
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
