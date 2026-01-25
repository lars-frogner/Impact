//! Physical quantities.

use crate::inertia::InertiaTensor;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    angle::{Angle, Radians},
    point::{Point3, Point3C},
    quaternion::{Quaternion, UnitQuaternion, UnitQuaternionC},
    vector::{UnitVector3, UnitVector3C, Vector3, Vector3C},
};
use roc_integration::roc;

/// A unit vector in 3D space.
pub type Direction = UnitVector3;
/// A unit vector in 3D space (compact).
pub type DirectionC = UnitVector3C;

/// A position in 3D space.
pub type Position = Point3;
/// A position in 3D space (compact).
pub type PositionC = Point3C;

/// A velocity in 3D space.
pub type Velocity = Vector3;
/// A velocity in 3D space (compact).
pub type VelocityC = Vector3C;

/// An orientation in 3D space.
pub type Orientation = UnitQuaternion;
/// An orientation in 3D space (compact).
pub type OrientationC = UnitQuaternionC;

/// A momentum in 3D space.
pub type Momentum = Vector3;
/// A momentum in 3D space (compact).
pub type MomentumC = Vector3C;

/// An angular momentum in 3D space.
pub type AngularMomentum = Vector3;
/// An angular momentum in 3D space (compact).
pub type AngularMomentumC = Vector3C;

/// An acceleration in 3D space.
pub type Acceleration = Vector3;
/// An acceleration in 3D space (compact).
pub type AccelerationC = Vector3C;

/// An angular acceleration in 3D space.
pub type AngularAcceleration = Vector3;
/// An angular acceleration in 3D space (compact).
pub type AngularAccelerationC = Vector3C;

/// A 3D force.
pub type Force = Vector3;
/// A 3D force (compact).
pub type ForceC = Vector3C;

/// A 3D torque.
pub type Torque = Vector3;
/// A 3D torque (compact).
pub type TorqueC = Vector3C;

/// A 3D impulse (momentum change).
pub type Impulse = Vector3;
/// A 3D impulse (momentum change) (compact).
pub type ImpulseC = Vector3C;

/// A 3D angular impulse (angular momentum change).
pub type AngularImpulse = Vector3;
/// A 3D angular impulse (angular momentum change) (compact).
pub type AngularImpulseC = Vector3C;

define_component_type! {
    /// A linear and angular velocity.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
    pub struct Motion {
        pub linear_velocity: VelocityC,
        pub angular_velocity: AngularVelocityC,
    }
}

/// An angular velocity in 3D space, represented by an axis of rotation and an
/// angular speed.
///
/// The axis is stored in a 128-bit SIMD register for efficient computation.
/// That leads to an extra 16 bytes in size (4 due to the padded axis and 12 due
/// to padding after the angular speed) and 16-byte alignment. For
/// cache-friendly storage, prefer the compact 4-byte aligned
/// [`AngularVelocityC`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngularVelocity {
    axis_of_rotation: Direction,
    angular_speed: Radians,
}

/// An angular velocity in 3D space, represented by an axis of rotation and an
/// angular speed. This is the "compact" version.
///
/// This type is primarily intended for compact storage inside other types and
/// collections. For computations, prefer the SIMD-friendly 16-byte aligned
/// [`AngularVelocity`].
#[roc(name = "AngularVelocity", parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct AngularVelocityC {
    axis_of_rotation: DirectionC,
    angular_speed: Radians,
}

#[roc]
impl Motion {
    #[roc(body = r#"
    {
        linear_velocity,
        angular_velocity,
    }"#)]
    #[inline]
    pub const fn new(linear_velocity: VelocityC, angular_velocity: AngularVelocityC) -> Self {
        Self {
            linear_velocity,
            angular_velocity,
        }
    }

    /// Motion with the given linear velocity and zero angular velocity.
    #[roc(body = "new(velocity, Physics.AngularVelocity.zero({}))")]
    #[inline]
    pub const fn linear(velocity: VelocityC) -> Self {
        Self::new(velocity, AngularVelocityC::zero())
    }

    /// Motion with with the given angular velocity and zero linear velocity.
    #[roc(body = "new(Vector3.zeros, velocity)")]
    #[inline]
    pub const fn angular(velocity: AngularVelocityC) -> Self {
        Self::new(VelocityC::zeros(), velocity)
    }

    /// No linear or angular motion.
    #[roc(body = "linear(Vector3.zeros)")]
    #[inline]
    pub const fn stationary() -> Self {
        Self::linear(VelocityC::zeros())
    }
}

impl AngularVelocity {
    /// Creates a new angular velocity with the given axis of rotation and
    /// angular speed.
    #[inline]
    pub const fn new(axis_of_rotation: Direction, angular_speed: Radians) -> Self {
        Self {
            axis_of_rotation,
            angular_speed,
        }
    }

    /// Creates a new angular velocity from the given angular velocity
    /// vector.
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

    /// Creates the angular velocity that would change the given first
    /// orientation to the given second orientation if applied for the given
    /// duration.
    #[inline]
    pub fn from_consecutive_orientations(
        first_orientation: &Orientation,
        second_orientation: &Orientation,
        duration: f32,
    ) -> Self {
        let difference = second_orientation * first_orientation.inverse();
        let (axis, angle) = difference.axis_angle();
        Self::new(axis, Radians(angle / duration))
    }

    /// Creates a new angular velocity with zero angular speed.
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
    pub const fn angular_speed(&self) -> Radians {
        self.angular_speed
    }

    /// Computes the corresponding angular velocity vector.
    #[inline]
    pub fn as_vector(&self) -> Vector3 {
        self.angular_speed.radians() * self.axis_of_rotation
    }

    /// Converts the tensor to the 4-byte aligned cache-friendly
    /// [`AngularVelocityC`].
    #[inline]
    pub fn compact(&self) -> AngularVelocityC {
        AngularVelocityC::new(self.axis_of_rotation.compact(), self.angular_speed)
    }
}

impl Default for AngularVelocity {
    fn default() -> Self {
        Self::zero()
    }
}

impl_binop!(
    Add,
    add,
    AngularVelocity,
    AngularVelocity,
    AngularVelocity,
    |a, b| { AngularVelocity::from_vector(a.as_vector() + b.as_vector()) }
);

impl_binop!(
    Sub,
    sub,
    AngularVelocity,
    AngularVelocity,
    AngularVelocity,
    |a, b| { AngularVelocity::from_vector(a.as_vector() - b.as_vector()) }
);

impl_binop_assign!(
    AddAssign,
    add_assign,
    AngularVelocity,
    AngularVelocity,
    |a, b| {
        *a = AngularVelocity::from_vector(a.as_vector() + b.as_vector());
    }
);

impl_binop_assign!(
    SubAssign,
    sub_assign,
    AngularVelocity,
    AngularVelocity,
    |a, b| {
        *a = AngularVelocity::from_vector(a.as_vector() - b.as_vector());
    }
);

impl_unary_op!(Neg, neg, AngularVelocity, AngularVelocity, |val| {
    AngularVelocity::new(-val.axis_of_rotation, val.angular_speed)
});

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

#[roc(dependencies=[Vector3C])]
impl AngularVelocityC {
    /// Creates a new angular velocity with the given axis of rotation and
    /// angular speed.
    #[roc(body = "{ axis_of_rotation, angular_speed }")]
    #[inline]
    pub const fn new(axis_of_rotation: DirectionC, angular_speed: Radians) -> Self {
        Self {
            axis_of_rotation,
            angular_speed,
        }
    }

    /// Creates a new angular velocity from the given angular velocity
    /// vector.
    #[roc(body = r#"
    when UnitVector3.try_from_and_get(angular_velocity_vector, 1e-15) is
        Some((axis_of_rotation, angular_speed)) -> new(axis_of_rotation, angular_speed)
        None -> zero({})
    "#)]
    #[inline]
    pub fn from_vector(angular_velocity_vector: Vector3C) -> Self {
        if let Some((axis_of_rotation, angular_speed)) =
            UnitVector3C::normalized_from_and_norm_if_above(angular_velocity_vector, f32::EPSILON)
        {
            Self::new(axis_of_rotation, Radians(angular_speed))
        } else {
            Self::zero()
        }
    }

    /// Creates a new angular velocity with zero angular speed.
    #[roc(body = "{ axis_of_rotation: UnitVector3.unit_y, angular_speed: 0.0 }")]
    #[inline]
    pub const fn zero() -> Self {
        Self {
            axis_of_rotation: UnitVector3C::unit_y(),
            angular_speed: Radians(0.0),
        }
    }

    /// Returns the axis of rotation.
    #[inline]
    pub const fn axis_of_rotation(&self) -> &DirectionC {
        &self.axis_of_rotation
    }

    /// Returns the angular speed.
    #[inline]
    pub const fn angular_speed(&self) -> Radians {
        self.angular_speed
    }

    /// Computes the corresponding angular velocity vector.
    #[roc(body = "Vector3.scale(self.axis_of_rotation, self.angular_speed)")]
    #[inline]
    pub fn as_vector(&self) -> Vector3C {
        self.angular_speed.radians() * self.axis_of_rotation
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly
    /// [`AngularVelocity`].
    #[inline]
    pub fn aligned(&self) -> AngularVelocity {
        AngularVelocity::new(self.axis_of_rotation.aligned(), self.angular_speed)
    }
}

impl Default for AngularVelocityC {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl_abs_diff_eq!(AngularVelocityC, |a, b, epsilon| {
    a.axis_of_rotation.abs_diff_eq(&b.axis_of_rotation, epsilon)
        && a.angular_speed.abs_diff_eq(&b.angular_speed, epsilon)
});

impl_relative_eq!(AngularVelocityC, |a, b, epsilon, max_relative| {
    a.axis_of_rotation
        .relative_eq(&b.axis_of_rotation, epsilon, max_relative)
        && a.angular_speed
            .relative_eq(&b.angular_speed, epsilon, max_relative)
});

/// Computes the quaternion representing the instantaneous time derivative of
/// the given orientation for a body with the given angular velocity.
#[inline]
pub fn compute_orientation_derivative(
    orientation: &Orientation,
    angular_velocity_vector: &Vector3,
) -> Quaternion {
    Quaternion::from_imag(0.5 * angular_velocity_vector) * orientation.as_quaternion()
}

/// Computes the angular velocity of a body with the given properties.
#[inline]
pub fn compute_angular_velocity(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    angular_momentum: &AngularMomentum,
) -> AngularVelocity {
    let inverse_world_space_inertia_tensor = inertia_tensor.inverse_rotated_matrix(orientation);
    AngularVelocity::from_vector(inverse_world_space_inertia_tensor * angular_momentum)
}

/// Computes the angular momentum of a body with the given properties.
#[inline]
pub fn compute_angular_momentum(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
) -> AngularMomentum {
    inertia_tensor.rotated_matrix(orientation) * angular_velocity.as_vector()
}

/// Computes the angular acceleration of a body with the given properties
/// when the body experiences the given torque around its center of mass.
#[inline]
pub fn compute_angular_acceleration(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    torque: &Torque,
) -> AngularAcceleration {
    inertia_tensor.inverse_rotated_matrix(orientation) * torque
}

/// Computes the total kinetic energy (translational and rotational) of a
/// body with the given properties.
#[inline]
pub fn compute_total_kinetic_energy(
    mass: f32,
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    velocity: &Velocity,
    angular_velocity: &AngularVelocity,
) -> f32 {
    compute_translational_kinetic_energy(mass, velocity)
        + compute_rotational_kinetic_energy(inertia_tensor, orientation, angular_velocity)
}

/// Computes the translational kinetic energy of a body with the given
/// properties.
#[inline]
pub fn compute_translational_kinetic_energy(mass: f32, velocity: &Velocity) -> f32 {
    0.5 * mass * velocity.norm_squared()
}

/// Computes the rotational kinetic energy of a body with the given properties.
#[inline]
pub fn compute_rotational_kinetic_energy(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
) -> f32 {
    let angular_momentum = compute_angular_momentum(inertia_tensor, orientation, angular_velocity);
    0.5 * angular_velocity.as_vector().dot(&angular_momentum)
}
