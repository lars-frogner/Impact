//! Physical quantities.

use crate::inertia::InertiaTensor;
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use impact_math::angle::{Angle, Radians};
use nalgebra::{Point3, Quaternion, Unit, UnitQuaternion, UnitVector3, Vector3};
use roc_integration::roc;
use std::ops::{Add, AddAssign, Sub, SubAssign};

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
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct AngularVelocity {
    axis_of_rotation: Direction,
    angular_speed: Radians<f32>,
}

/// A unit vector in 3D space.
pub type Direction = Unit<Vector3<f32>>;

/// A position in 3D space.
pub type Position = Point3<f32>;

/// A velocity in 3D space.
pub type Velocity = Vector3<f32>;

/// An orientation in 3D space.
pub type Orientation = UnitQuaternion<f32>;

/// A momentum in 3D space.
pub type Momentum = Vector3<f32>;

/// An angular momentum in 3D space.
pub type AngularMomentum = Vector3<f32>;

/// An acceleration in 3D space.
pub type Acceleration = Vector3<f32>;

/// An angular acceleration in 3D space.
pub type AngularAcceleration = Vector3<f32>;

/// A 3D force.
pub type Force = Vector3<f32>;

/// A 3D torque.
pub type Torque = Vector3<f32>;

#[roc]
impl Motion {
    #[roc(body = r#"
    {
        linear_velocity,
        angular_velocity,
    }"#)]
    pub fn new(linear_velocity: Velocity, angular_velocity: AngularVelocity) -> Self {
        Self {
            linear_velocity,
            angular_velocity,
        }
    }

    /// Motion with the given linear velocity and zero angular velocity.
    #[roc(body = "new(velocity, Physics.AngularVelocity.zero({}))")]
    pub fn linear(velocity: Velocity) -> Self {
        Self::new(velocity, AngularVelocity::zero())
    }

    /// Motion with with the given angular velocity and zero linear velocity.
    #[roc(body = "new(Vector3.zero, velocity)")]
    pub fn angular(velocity: AngularVelocity) -> Self {
        Self::new(Velocity::zeros(), velocity)
    }

    /// No linear or angular motion.
    #[roc(body = "linear(Vector3.zero)")]
    pub fn stationary() -> Self {
        Self::linear(Velocity::zeros())
    }
}

#[roc(dependencies=[Vector3<f32>])]
impl AngularVelocity {
    /// Creates a new [`AngularVelocity`] with the given axis of rotation and
    /// angular speed.
    #[roc(body = "{ axis_of_rotation, angular_speed }")]
    pub fn new(axis_of_rotation: Direction, angular_speed: Radians<f32>) -> Self {
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
    pub fn from_vector(angular_velocity_vector: Vector3<f32>) -> Self {
        if let Some((axis_of_rotation, angular_speed)) =
            UnitVector3::try_new_and_get(angular_velocity_vector, f32::EPSILON)
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
        duration: f32,
    ) -> Self {
        let difference = second_orientation * first_orientation.inverse();
        if let Some((axis, angle)) = difference.axis_angle() {
            Self::new(axis, Radians(angle / duration))
        } else {
            Self::zero()
        }
    }

    /// Creates a new [`AngularVelocity`] with zero angular speed.
    #[roc(body = "{ axis_of_rotation: UnitVector3.y_axis, angular_speed: 0.0 }")]
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
    pub fn angular_speed(&self) -> Radians<f32> {
        self.angular_speed
    }

    /// Computes the corresponding angular velocity vector.
    pub fn as_vector(&self) -> Vector3<f32> {
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
    type Epsilon = <f32 as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
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
    angular_velocity_vector: &Vector3<f32>,
) -> Quaternion<f32> {
    Quaternion::from_imag(0.5 * angular_velocity_vector) * orientation.as_ref()
}

/// Computes the [`AngularVelocity`] of a body with the given properties.
pub fn compute_angular_velocity(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    angular_momentum: &AngularMomentum,
) -> AngularVelocity {
    let inverse_world_space_inertia_tensor = inertia_tensor.inverse_rotated_matrix(orientation);
    AngularVelocity::from_vector(inverse_world_space_inertia_tensor * angular_momentum)
}

/// Computes the [`AngularMomentum`] of a body with the given properties.
pub fn compute_angular_momentum(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
) -> AngularMomentum {
    inertia_tensor.rotated_matrix(orientation) * angular_velocity.as_vector()
}

/// Computes the [`AngularAcceleration`] of a body with the given properties
/// when the body experiences the given [`Torque`] around its center of mass.
pub fn compute_angular_acceleration(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    torque: &Torque,
) -> AngularAcceleration {
    inertia_tensor.inverse_rotated_matrix(orientation) * torque
}

/// Computes the total kinetic energy (translational and rotational) of a
/// body with the given properties.
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
pub fn compute_translational_kinetic_energy(mass: f32, velocity: &Velocity) -> f32 {
    0.5 * mass * velocity.norm_squared()
}

/// Computes the rotational kinetic energy of a body with the given properties.
pub fn compute_rotational_kinetic_energy(
    inertia_tensor: &InertiaTensor,
    orientation: &Orientation,
    angular_velocity: &AngularVelocity,
) -> f32 {
    let angular_momentum = compute_angular_momentum(inertia_tensor, orientation, angular_velocity);
    0.5 * angular_velocity.as_vector().dot(&angular_momentum)
}
