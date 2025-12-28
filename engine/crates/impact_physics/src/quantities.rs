//! Physical quantities.

use crate::inertia::InertiaTensorA;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    angle::{Angle, Radians},
    point::{Point3, Point3A},
    quaternion::{QuaternionA, UnitQuaternion, UnitQuaternionA},
    vector::{UnitVector3, UnitVector3A, Vector3, Vector3A},
};
use roc_integration::roc;

define_component_type! {
    /// A linear and angular velocity.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
    pub struct Motion {
        pub linear_velocity: Velocity,
        pub angular_velocity: AngularVelocity,
    }
}

/// An angular velocity in 3D space, represented by an axis of rotation and an
/// angular speed.
///
/// This type is primarily intended for compact storage inside other types and
/// collections. For computations, prefer the SIMD-friendly 16-byte aligned
/// [`AngularVelocityA`].
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct AngularVelocity {
    axis_of_rotation: Direction,
    angular_speed: Radians<f32>,
}

/// An angular velocity in 3D space, represented by an axis of rotation and an
/// angular speed.
///
/// The axis is stored in a 128-bit SIMD register for efficient computation.
/// That leads to an extra 16 bytes in size (4 due to the padded axis and 12 due
/// to padding after the angular speed) and 16-byte alignment. For
/// cache-friendly storage, prefer [`AngularVelocity`].
#[derive(Clone, Debug, PartialEq)]
pub struct AngularVelocityA {
    axis_of_rotation: DirectionA,
    angular_speed: Radians<f32>,
}

/// A unit vector in 3D space.
pub type Direction = UnitVector3;
/// A unit vector in 3D space (aligned).
pub type DirectionA = UnitVector3A;

/// A position in 3D space.
pub type Position = Point3;
/// A position in 3D space (aligned).
pub type PositionA = Point3A;

/// A velocity in 3D space.
pub type Velocity = Vector3;
/// A velocity in 3D space (aligned).
pub type VelocityA = Vector3A;

/// An orientation in 3D space.
pub type Orientation = UnitQuaternion;
/// An orientation in 3D space (aligned).
pub type OrientationA = UnitQuaternionA;

/// A momentum in 3D space.
pub type Momentum = Vector3;
/// A momentum in 3D space (aligned).
pub type MomentumA = Vector3A;

/// An angular momentum in 3D space.
pub type AngularMomentum = Vector3;
/// An angular momentum in 3D space (aligned).
pub type AngularMomentumA = Vector3A;

/// An acceleration in 3D space.
pub type Acceleration = Vector3;
/// An acceleration in 3D space (aligned).
pub type AccelerationA = Vector3A;

/// An angular acceleration in 3D space.
pub type AngularAcceleration = Vector3;
/// An angular acceleration in 3D space (aligned).
pub type AngularAccelerationA = Vector3A;

/// A 3D force.
pub type Force = Vector3;
/// A 3D force (aligned).
pub type ForceA = Vector3A;

/// A 3D torque.
pub type Torque = Vector3;
/// A 3D torque (aligned).
pub type TorqueA = Vector3A;

#[roc]
impl Motion {
    #[roc(body = r#"
    {
        linear_velocity,
        angular_velocity,
    }"#)]
    #[inline]
    pub const fn new(linear_velocity: Velocity, angular_velocity: AngularVelocity) -> Self {
        Self {
            linear_velocity,
            angular_velocity,
        }
    }

    /// Motion with the given linear velocity and zero angular velocity.
    #[roc(body = "new(velocity, Physics.AngularVelocity.zero({}))")]
    #[inline]
    pub const fn linear(velocity: Velocity) -> Self {
        Self::new(velocity, AngularVelocity::zero())
    }

    /// Motion with with the given angular velocity and zero linear velocity.
    #[roc(body = "new(Vector3.zero, velocity)")]
    #[inline]
    pub const fn angular(velocity: AngularVelocity) -> Self {
        Self::new(Velocity::zeros(), velocity)
    }

    /// No linear or angular motion.
    #[roc(body = "linear(Vector3.zero)")]
    #[inline]
    pub const fn stationary() -> Self {
        Self::linear(Velocity::zeros())
    }
}

#[roc(dependencies=[Vector3])]
impl AngularVelocity {
    /// Creates a new [`AngularVelocity`] with the given axis of rotation and
    /// angular speed.
    #[roc(body = "{ axis_of_rotation, angular_speed }")]
    #[inline]
    pub const fn new(axis_of_rotation: Direction, angular_speed: Radians<f32>) -> Self {
        Self {
            axis_of_rotation,
            angular_speed,
        }
    }

    /// Creates a new [`AngularVelocity`] from the given angular velocity
    /// vector.
    #[roc(body = r#"
    when UnitVector3.try_from_and_get(angular_velocity_vector, 1e-15) is
        Some((axis_of_rotation, angular_speed)) -> new(axis_of_rotation, angular_speed)
        None -> zero({})
    "#)]
    #[inline]
    pub fn from_vector(angular_velocity_vector: Vector3) -> Self {
        if let Some((axis_of_rotation, angular_speed)) =
            UnitVector3::normalized_from_and_norm_if_above(angular_velocity_vector, f32::EPSILON)
        {
            Self::new(axis_of_rotation, Radians(angular_speed))
        } else {
            Self::zero()
        }
    }

    /// Creates a new [`AngularVelocity`] with zero angular speed.
    #[roc(body = "{ axis_of_rotation: UnitVector3.y_axis, angular_speed: 0.0 }")]
    #[inline]
    pub const fn zero() -> Self {
        Self {
            axis_of_rotation: UnitVector3::unit_y(),
            angular_speed: Radians(0.0),
        }
    }

    /// Returns the axis of rotation.
    #[inline]
    pub const fn axis_of_rotation(&self) -> &Direction {
        &self.axis_of_rotation
    }

    /// Returns the angular speed.
    #[inline]
    pub const fn angular_speed(&self) -> Radians<f32> {
        self.angular_speed
    }

    /// Computes the corresponding angular velocity vector.
    #[inline]
    pub fn as_vector(&self) -> Vector3 {
        self.angular_speed.radians() * self.axis_of_rotation
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly
    /// [`AngularVelocityA`].
    #[inline]
    pub fn aligned(&self) -> AngularVelocityA {
        AngularVelocityA::new(self.axis_of_rotation.aligned(), self.angular_speed)
    }
}

impl Default for AngularVelocity {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl_abs_diff_eq!(AngularVelocity, |a, b, epsilon| {
    a.axis_of_rotation.abs_diff_eq(&b.axis_of_rotation, epsilon)
        && a.angular_speed.abs_diff_eq(&b.angular_speed, epsilon)
});

impl_relative_eq!(AngularVelocity, |a, b, epsilon, max_relative| {
    a.axis_of_rotation
        .relative_eq(&b.axis_of_rotation, epsilon, max_relative)
        && a.angular_speed
            .relative_eq(&b.angular_speed, epsilon, max_relative)
});

impl AngularVelocityA {
    /// Creates a new [`AngularVelocityA`] with the given axis of rotation and
    /// angular speed.
    #[inline]
    pub const fn new(axis_of_rotation: DirectionA, angular_speed: Radians<f32>) -> Self {
        Self {
            axis_of_rotation,
            angular_speed,
        }
    }

    /// Creates a new [`AngularVelocityA`] from the given angular velocity
    /// vector.
    #[inline]
    pub fn from_vector(angular_velocity_vector: Vector3A) -> Self {
        if let Some((axis_of_rotation, angular_speed)) =
            UnitVector3A::normalized_from_and_norm_if_above(angular_velocity_vector, f32::EPSILON)
        {
            Self::new(axis_of_rotation, Radians(angular_speed))
        } else {
            Self::zero()
        }
    }

    /// Creates the [`AngularVelocityA`] that would change the given first
    /// orientation to the given second orientation if applied for the given
    /// duration.
    #[inline]
    pub fn from_consecutive_orientations(
        first_orientation: &OrientationA,
        second_orientation: &OrientationA,
        duration: f32,
    ) -> Self {
        let difference = second_orientation * first_orientation.inverse();
        let (axis, angle) = difference.axis_angle();
        Self::new(axis, Radians(angle / duration))
    }

    /// Creates a new [`AngularVelocityA`] with zero angular speed.
    #[inline]
    pub const fn zero() -> Self {
        Self {
            axis_of_rotation: UnitVector3A::unit_y(),
            angular_speed: Radians(0.0),
        }
    }

    /// Returns the axis of rotation.
    #[inline]
    pub const fn axis_of_rotation(&self) -> &DirectionA {
        &self.axis_of_rotation
    }

    /// Returns the angular speed.
    #[inline]
    pub const fn angular_speed(&self) -> Radians<f32> {
        self.angular_speed
    }

    /// Computes the corresponding angular velocity vector.
    #[inline]
    pub fn as_vector(&self) -> Vector3A {
        self.angular_speed.radians() * self.axis_of_rotation
    }

    /// Converts the tensor to the 4-byte aligned cache-friendly
    /// [`AngularVelocity`].
    #[inline]
    pub fn unaligned(&self) -> AngularVelocity {
        AngularVelocity::new(self.axis_of_rotation.unaligned(), self.angular_speed)
    }
}

impl Default for AngularVelocityA {
    fn default() -> Self {
        Self::zero()
    }
}

impl_binop!(
    Add,
    add,
    AngularVelocityA,
    AngularVelocityA,
    AngularVelocityA,
    |a, b| { AngularVelocityA::from_vector(a.as_vector() + b.as_vector()) }
);

impl_binop!(
    Sub,
    sub,
    AngularVelocityA,
    AngularVelocityA,
    AngularVelocityA,
    |a, b| { AngularVelocityA::from_vector(a.as_vector() - b.as_vector()) }
);

impl_binop_assign!(
    AddAssign,
    add_assign,
    AngularVelocityA,
    AngularVelocityA,
    |a, b| {
        *a = AngularVelocityA::from_vector(a.as_vector() + b.as_vector());
    }
);

impl_binop_assign!(
    SubAssign,
    sub_assign,
    AngularVelocityA,
    AngularVelocityA,
    |a, b| {
        *a = AngularVelocityA::from_vector(a.as_vector() - b.as_vector());
    }
);

impl_unary_op!(Neg, neg, AngularVelocityA, AngularVelocityA, |val| {
    AngularVelocityA::new(-val.axis_of_rotation, val.angular_speed)
});

impl_abs_diff_eq!(AngularVelocityA, |a, b, epsilon| {
    a.axis_of_rotation.abs_diff_eq(&b.axis_of_rotation, epsilon)
        && a.angular_speed.abs_diff_eq(&b.angular_speed, epsilon)
});

impl_relative_eq!(AngularVelocityA, |a, b, epsilon, max_relative| {
    a.axis_of_rotation
        .relative_eq(&b.axis_of_rotation, epsilon, max_relative)
        && a.angular_speed
            .relative_eq(&b.angular_speed, epsilon, max_relative)
});

/// Computes the quaternion representing the instantaneous time derivative of
/// the given orientation for a body with the given angular velocity.
#[inline]
pub fn compute_orientation_derivative(
    orientation: &OrientationA,
    angular_velocity_vector: &Vector3A,
) -> QuaternionA {
    QuaternionA::from_imag(0.5 * angular_velocity_vector) * orientation.as_quaternion()
}

/// Computes the angular velocity of a body with the given properties.
#[inline]
pub fn compute_angular_velocity(
    inertia_tensor: &InertiaTensorA,
    orientation: &OrientationA,
    angular_momentum: &AngularMomentumA,
) -> AngularVelocityA {
    let inverse_world_space_inertia_tensor = inertia_tensor.inverse_rotated_matrix(orientation);
    AngularVelocityA::from_vector(inverse_world_space_inertia_tensor * angular_momentum)
}

/// Computes the angular momentum of a body with the given properties.
#[inline]
pub fn compute_angular_momentum(
    inertia_tensor: &InertiaTensorA,
    orientation: &OrientationA,
    angular_velocity: &AngularVelocityA,
) -> AngularMomentumA {
    inertia_tensor.rotated_matrix(orientation) * angular_velocity.as_vector()
}

/// Computes the angular acceleration of a body with the given properties
/// when the body experiences the given torque around its center of mass.
#[inline]
pub fn compute_angular_acceleration(
    inertia_tensor: &InertiaTensorA,
    orientation: &OrientationA,
    torque: &TorqueA,
) -> AngularAccelerationA {
    inertia_tensor.inverse_rotated_matrix(orientation) * torque
}

/// Computes the total kinetic energy (translational and rotational) of a
/// body with the given properties.
#[inline]
pub fn compute_total_kinetic_energy(
    mass: f32,
    inertia_tensor: &InertiaTensorA,
    orientation: &OrientationA,
    velocity: &VelocityA,
    angular_velocity: &AngularVelocityA,
) -> f32 {
    compute_translational_kinetic_energy(mass, velocity)
        + compute_rotational_kinetic_energy(inertia_tensor, orientation, angular_velocity)
}

/// Computes the translational kinetic energy of a body with the given
/// properties.
#[inline]
pub fn compute_translational_kinetic_energy(mass: f32, velocity: &VelocityA) -> f32 {
    0.5 * mass * velocity.norm_squared()
}

/// Computes the rotational kinetic energy of a body with the given properties.
#[inline]
pub fn compute_rotational_kinetic_energy(
    inertia_tensor: &InertiaTensorA,
    orientation: &OrientationA,
    angular_velocity: &AngularVelocityA,
) -> f32 {
    let angular_momentum = compute_angular_momentum(inertia_tensor, orientation, angular_velocity);
    0.5 * angular_velocity.as_vector().dot(&angular_momentum)
}
