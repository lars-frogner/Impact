//! Different units for angles.

use crate::consts::f32::{FRAC_1_PI, PI};
use approx::{AbsDiffEq, RelativeEq};
use bytemuck::{Pod, Zeroable};
use std::{
    cmp::Ordering,
    ops::{Add, Div, Mul, Sub},
};

/// Represents an angle.
pub trait Angle: Copy {
    /// Creates a zero angle.
    fn zero() -> Self;

    /// Returns the angle as degrees.
    fn as_degrees(self) -> Degrees;

    /// Returns the angle as radians.
    fn as_radians(self) -> Radians;

    /// Returns the value of the angle in degrees.
    fn degrees(self) -> f32;

    /// Returns the value of the angle in radians.
    fn radians(self) -> f32;
}

// An angle in degrees.
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Zeroable, Pod)]
pub struct Degrees(pub f32);

// An angle in radians.
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Zeroable, Pod)]
pub struct Radians(pub f32);

impl Degrees {
    fn value(self) -> f32 {
        self.0
    }
}

impl Radians {
    fn value(self) -> f32 {
        self.0
    }
}

impl Angle for Degrees {
    fn zero() -> Self {
        Self(0.0)
    }

    fn as_degrees(self) -> Degrees {
        self
    }

    fn as_radians(self) -> Radians {
        Radians::from(self)
    }

    fn degrees(self) -> f32 {
        self.value()
    }

    fn radians(self) -> f32 {
        Radians::from(self).value()
    }
}

impl Angle for Radians {
    fn zero() -> Self {
        Self(0.0)
    }

    fn as_degrees(self) -> Degrees {
        Degrees::from(self)
    }

    fn as_radians(self) -> Radians {
        self
    }

    fn degrees(self) -> f32 {
        Degrees::from(self).value()
    }

    fn radians(self) -> f32 {
        self.value()
    }
}

impl From<Radians> for Degrees {
    fn from(rad: Radians) -> Self {
        Self(radians_to_degrees(rad.value()))
    }
}

impl From<Degrees> for Radians {
    fn from(deg: Degrees) -> Self {
        Self(degrees_to_radians(deg.value()))
    }
}

impl Add for Degrees {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.value() + rhs.value())
    }
}

impl Add for Radians {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.value() + rhs.value())
    }
}

impl Add<Radians> for Degrees {
    type Output = Self;
    fn add(self, rhs: Radians) -> Self {
        Self(self.value() + Self::from(rhs).value())
    }
}

impl Add<Degrees> for Radians {
    type Output = Self;
    fn add(self, rhs: Degrees) -> Self {
        Self(self.value() + Self::from(rhs).value())
    }
}

impl Sub for Degrees {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.value() - rhs.value())
    }
}

impl Sub for Radians {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.value() - rhs.value())
    }
}

impl Sub<Radians> for Degrees {
    type Output = Self;
    fn sub(self, rhs: Radians) -> Self {
        Self(self.value() - Self::from(rhs).value())
    }
}

impl Sub<Degrees> for Radians {
    type Output = Self;
    fn sub(self, rhs: Degrees) -> Self {
        Self(self.value() - Self::from(rhs).value())
    }
}

impl Mul<f32> for Degrees {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self(self.value() * rhs)
    }
}

impl Mul<f32> for Radians {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self(self.value() * rhs)
    }
}

impl Div<f32> for Degrees {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self(self.value() / rhs)
    }
}

impl Div<f32> for Radians {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self(self.value() / rhs)
    }
}

impl PartialEq<Radians> for Degrees {
    fn eq(&self, rhs: &Radians) -> bool {
        self.value() == Self::from(*rhs).value()
    }
}

impl PartialEq<Degrees> for Radians {
    fn eq(&self, rhs: &Degrees) -> bool {
        self.value() == Self::from(*rhs).value()
    }
}

impl PartialOrd<Radians> for Degrees {
    fn partial_cmp(&self, rhs: &Radians) -> Option<Ordering> {
        self.value().partial_cmp(&Self::from(*rhs).value())
    }
}

impl PartialOrd<Degrees> for Radians {
    fn partial_cmp(&self, rhs: &Degrees) -> Option<Ordering> {
        self.value().partial_cmp(&Self::from(*rhs).value())
    }
}

impl AbsDiffEq for Degrees {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        f32::abs_diff_eq(&self.value(), &other.value(), epsilon)
    }
}

impl AbsDiffEq for Radians {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        f32::abs_diff_eq(&self.value(), &other.value(), epsilon)
    }
}

impl AbsDiffEq<Radians> for Degrees {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Radians, epsilon: f32) -> bool {
        f32::abs_diff_eq(&self.value(), &other.degrees(), epsilon)
    }
}

impl AbsDiffEq<Degrees> for Radians {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Degrees, epsilon: f32) -> bool {
        f32::abs_diff_eq(&self.value(), &other.radians(), epsilon)
    }
}

impl RelativeEq for Degrees {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        f32::relative_eq(&self.value(), &other.value(), epsilon, max_relative)
    }
}

impl RelativeEq<Radians> for Degrees {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(&self, other: &Radians, epsilon: f32, max_relative: f32) -> bool {
        f32::relative_eq(&self.value(), &other.degrees(), epsilon, max_relative)
    }
}

impl RelativeEq<Degrees> for Radians {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(&self, other: &Degrees, epsilon: f32, max_relative: f32) -> bool {
        f32::relative_eq(&self.value(), &other.radians(), epsilon, max_relative)
    }
}

impl RelativeEq for Radians {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        f32::relative_eq(&self.value(), &other.value(), epsilon, max_relative)
    }
}

roc_integration::impl_roc_for_library_provided_primitives! {
//  Type       Pkg   Parents  Module   Roc name  Postfix  Precision
    Radians => core, None,    Radians, Radians,  None,  PrecisionIrrelevant,
    Degrees => core, None,    Degrees, Degrees,  None,  PrecisionIrrelevant,
}

pub fn radians_to_degrees(radians: f32) -> f32 {
    radians * (180.0 * FRAC_1_PI)
}

pub fn degrees_to_radians(degrees: f32) -> f32 {
    degrees * (PI / 180.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

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
