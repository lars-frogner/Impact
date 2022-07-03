//! Representations of angles.

use crate::num::Float;
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
        Self(rad.value() * F::from_f64(180.0).unwrap() * F::FRAC_1_PI())
    }
}

impl<F: Float> From<Degrees<F>> for Radians<F> {
    fn from(deg: Degrees<F>) -> Self {
        Self(deg.value() * F::PI() / F::from_f64(180.0).unwrap())
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

impl<T> AbsDiffEq for Degrees<T>
where
    T: Copy + AbsDiffEq,
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

impl<T> AbsDiffEq for Radians<T>
where
    T: Copy + AbsDiffEq,
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

impl<T: Float> AbsDiffEq<Radians<T>> for Degrees<T> {
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> T::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Radians<T>, epsilon: T::Epsilon) -> bool {
        T::abs_diff_eq(&self.value(), &other.degrees(), epsilon)
    }
}

impl<T: Float> AbsDiffEq<Degrees<T>> for Radians<T> {
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> T::Epsilon {
        T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Degrees<T>, epsilon: T::Epsilon) -> bool {
        T::abs_diff_eq(&self.value(), &other.radians(), epsilon)
    }
}

impl<T> RelativeEq for Degrees<T>
where
    T: Copy + RelativeEq,
    T::Epsilon: Copy,
{
    fn default_max_relative() -> T::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: T::Epsilon, max_relative: T::Epsilon) -> bool {
        T::relative_eq(&self.value(), &other.value(), epsilon, max_relative)
    }
}

impl<T: Float> RelativeEq<Radians<T>> for Degrees<T> {
    fn default_max_relative() -> T::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Radians<T>,
        epsilon: T::Epsilon,
        max_relative: T::Epsilon,
    ) -> bool {
        T::relative_eq(&self.value(), &other.degrees(), epsilon, max_relative)
    }
}

impl<T: Float> RelativeEq<Degrees<T>> for Radians<T> {
    fn default_max_relative() -> T::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Degrees<T>,
        epsilon: T::Epsilon,
        max_relative: T::Epsilon,
    ) -> bool {
        T::relative_eq(&self.value(), &other.radians(), epsilon, max_relative)
    }
}

impl<T> RelativeEq for Radians<T>
where
    T: Copy + RelativeEq,
    T::Epsilon: Copy,
{
    fn default_max_relative() -> T::Epsilon {
        T::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: T::Epsilon, max_relative: T::Epsilon) -> bool {
        T::relative_eq(&self.value(), &other.value(), epsilon, max_relative)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use std::f64::consts::PI;

    #[test]
    fn degrees_to_radians_for_special_angles_work() {
        assert_abs_diff_eq!(Degrees(0.0).radians(), 0.0);

        assert_abs_diff_eq!(Degrees(90.0).radians(), PI / 2.0);
        assert_abs_diff_eq!(Degrees(180.0).radians(), PI);
        assert_abs_diff_eq!(Degrees(270.0).radians(), 3.0 * PI / 2.0);
        assert_abs_diff_eq!(Degrees(360.0).radians(), 2.0 * PI);

        assert_abs_diff_eq!(Degrees(-90.0).radians(), -PI / 2.0);
        assert_abs_diff_eq!(Degrees(-180.0).radians(), -PI);
        assert_abs_diff_eq!(Degrees(-270.0).radians(), -3.0 * PI / 2.0);
        assert_abs_diff_eq!(Degrees(-360.0).radians(), -2.0 * PI);
    }

    #[test]
    fn radians_to_degrees_for_special_angles_work() {
        assert_abs_diff_eq!(Radians(0.0).degrees(), 0.0);

        assert_abs_diff_eq!(Radians(PI / 2.0).degrees(), 90.0);
        assert_abs_diff_eq!(Radians(PI).degrees(), 180.0);
        assert_abs_diff_eq!(Radians(3.0 * PI / 2.0).degrees(), 270.0);
        assert_abs_diff_eq!(Radians(2.0 * PI).degrees(), 360.0);

        assert_abs_diff_eq!(Radians(-PI / 2.0).degrees(), -90.0);
        assert_abs_diff_eq!(Radians(-PI).degrees(), -180.0);
        assert_abs_diff_eq!(Radians(-3.0 * PI / 2.0).degrees(), -270.0);
        assert_abs_diff_eq!(Radians(-2.0 * PI).degrees(), -360.0);
    }

    #[test]
    fn degree_ops_work() {
        assert_abs_diff_eq!(Degrees(42.0) + Degrees(30.0), Degrees(72.0));
        assert_abs_diff_eq!(Degrees(42.0) - Degrees(30.0), Degrees(12.0));
        assert_abs_diff_eq!(Degrees(42.0) * 2.5, Degrees(105.0));
        assert_abs_diff_eq!(Degrees(42.0) / 4.0, Degrees(10.5));
    }

    #[test]
    fn radian_ops_work() {
        assert_abs_diff_eq!(Radians(42.0) + Radians(30.0), Radians(72.0));
        assert_abs_diff_eq!(Radians(42.0) - Radians(30.0), Radians(12.0));
        assert_abs_diff_eq!(Radians(42.0) * 2.5, Radians(105.0));
        assert_abs_diff_eq!(Radians(42.0) / 4.0, Radians(10.5));
    }

    #[test]
    fn mixed_degree_radian_ops_work() {
        assert_abs_diff_eq!(Degrees(45.0) + Radians(PI / 2.0), Degrees(135.0));
        assert_abs_diff_eq!(Radians(PI / 2.0) + Degrees(45.0), Radians(3.0 * PI / 4.0));
        assert_abs_diff_eq!(Degrees(45.0) - Radians(PI / 2.0), Degrees(-45.0));
        assert_abs_diff_eq!(Radians(PI / 2.0) - Degrees(45.0), Radians(PI / 4.0));

        assert_eq!(Degrees(0.0), Radians(0.0));
        assert!(Degrees(42.0) > Radians(0.0));
        assert!(Degrees(42.0) < Radians(PI));

        assert_eq!(Radians(0.0), Degrees(0.0));
        assert!(Radians(PI) > Degrees(0.0));
        assert!(Radians(PI) < Degrees(360.0));
    }
}
