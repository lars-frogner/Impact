//! Representations of angles.

use crate::{float_from, num::Float};
use approx::{AbsDiffEq, RelativeEq};
use std::{
    cmp::Ordering,
    ops::{Add, Div, Mul, Sub},
};

/// Represents an angle.
pub trait Angle<F>: Copy {
    /// Creates a zero angle.
    fn zero() -> Self;

    /// Returns the angle as degrees.
    fn as_degrees(self) -> Degrees<F>;

    /// Returns the angle as radians.
    fn as_radians(self) -> Radians<F>;

    /// Returns the value of the angle in degrees.
    fn degrees(self) -> F;

    /// Returns the value of the angle in radians.
    fn radians(self) -> F;
}

// An angle in degrees.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Degrees<F>(pub F);

// An angle in radians.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Radians<F>(pub F);

impl<F> Degrees<F> {
    fn value(self) -> F {
        self.0
    }
}

impl<F> Radians<F> {
    fn value(self) -> F {
        self.0
    }
}

impl<F: Float> Angle<F> for Degrees<F> {
    fn zero() -> Self {
        Self(F::zero())
    }

    fn as_degrees(self) -> Degrees<F> {
        self
    }

    fn as_radians(self) -> Radians<F> {
        Radians::from(self)
    }

    fn degrees(self) -> F {
        self.value()
    }

    fn radians(self) -> F {
        Radians::from(self).value()
    }
}

impl<F: Float> Angle<F> for Radians<F> {
    fn zero() -> Self {
        Self(F::zero())
    }

    fn as_degrees(self) -> Degrees<F> {
        Degrees::from(self)
    }

    fn as_radians(self) -> Radians<F> {
        self
    }

    fn degrees(self) -> F {
        Degrees::from(self).value()
    }

    fn radians(self) -> F {
        self.value()
    }
}

impl<F: Float> From<Radians<F>> for Degrees<F> {
    fn from(rad: Radians<F>) -> Self {
        Self(rad.value() * float_from!(F, 180.0) * F::FRAC_1_PI())
    }
}

impl<F: Float> From<Degrees<F>> for Radians<F> {
    fn from(deg: Degrees<F>) -> Self {
        Self(deg.value() * F::PI() / float_from!(F, 180.0))
    }
}

impl<F: Add<Output = F>> Add for Degrees<F> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.value() + rhs.value())
    }
}

impl<F: Add<Output = F>> Add for Radians<F> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.value() + rhs.value())
    }
}

impl<F: Float> Add<Radians<F>> for Degrees<F> {
    type Output = Self;
    fn add(self, rhs: Radians<F>) -> Self {
        Self(self.value() + Self::from(rhs).value())
    }
}

impl<F: Float> Add<Degrees<F>> for Radians<F> {
    type Output = Self;
    fn add(self, rhs: Degrees<F>) -> Self {
        Self(self.value() + Self::from(rhs).value())
    }
}

impl<F: Sub<Output = F>> Sub for Degrees<F> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.value() - rhs.value())
    }
}

impl<F: Sub<Output = F>> Sub for Radians<F> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.value() - rhs.value())
    }
}

impl<F: Float> Sub<Radians<F>> for Degrees<F> {
    type Output = Self;
    fn sub(self, rhs: Radians<F>) -> Self {
        Self(self.value() - Self::from(rhs).value())
    }
}

impl<F: Float> Sub<Degrees<F>> for Radians<F> {
    type Output = Self;
    fn sub(self, rhs: Degrees<F>) -> Self {
        Self(self.value() - Self::from(rhs).value())
    }
}

impl<F: Mul<Output = F>> Mul<F> for Degrees<F> {
    type Output = Self;
    fn mul(self, rhs: F) -> Self {
        Self(self.value() * rhs)
    }
}

impl<F: Mul<Output = F>> Mul<F> for Radians<F> {
    type Output = Self;
    fn mul(self, rhs: F) -> Self {
        Self(self.value() * rhs)
    }
}

impl<F: Div<Output = F>> Div<F> for Degrees<F> {
    type Output = Self;
    fn div(self, rhs: F) -> Self {
        Self(self.value() / rhs)
    }
}

impl<F: Div<Output = F>> Div<F> for Radians<F> {
    type Output = Self;
    fn div(self, rhs: F) -> Self {
        Self(self.value() / rhs)
    }
}

impl<F: Float> PartialEq<Radians<F>> for Degrees<F> {
    fn eq(&self, rhs: &Radians<F>) -> bool {
        self.value() == Self::from(*rhs).value()
    }
}

impl<F: Float> PartialEq<Degrees<F>> for Radians<F> {
    fn eq(&self, rhs: &Degrees<F>) -> bool {
        self.value() == Self::from(*rhs).value()
    }
}

impl<F: Float> PartialOrd<Radians<F>> for Degrees<F> {
    fn partial_cmp(&self, rhs: &Radians<F>) -> Option<Ordering> {
        self.value().partial_cmp(&Self::from(*rhs).value())
    }
}

impl<F: Float> PartialOrd<Degrees<F>> for Radians<F> {
    fn partial_cmp(&self, rhs: &Degrees<F>) -> Option<Ordering> {
        self.value().partial_cmp(&Self::from(*rhs).value())
    }
}

impl<T: Copy + AbsDiffEq> AbsDiffEq for Degrees<T>
where
    T::Epsilon: Copy,
{
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> T::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: T::Epsilon) -> bool {
        T::abs_diff_eq(&self.value(), &other.value(), epsilon)
    }
}

impl<T: Copy + AbsDiffEq> AbsDiffEq for Radians<T>
where
    T::Epsilon: Copy,
{
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> T::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: T::Epsilon) -> bool {
        T::abs_diff_eq(&self.value(), &other.value(), epsilon)
    }
}

impl<T: Copy + RelativeEq> RelativeEq for Degrees<T>
where
    T::Epsilon: Copy,
{
    fn default_max_relative() -> T::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: T::Epsilon, max_relative: T::Epsilon) -> bool {
        T::relative_eq(&self.value(), &other.value(), epsilon, max_relative)
    }
}

impl<T: Copy + RelativeEq> RelativeEq for Radians<T>
where
    T::Epsilon: Copy,
{
    fn default_max_relative() -> T::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: T::Epsilon, max_relative: T::Epsilon) -> bool {
        T::relative_eq(&self.value(), &other.value(), epsilon, max_relative)
    }
}
