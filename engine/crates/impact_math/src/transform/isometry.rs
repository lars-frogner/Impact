//! Isometry transforms.

use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};

type Point3 = nalgebra::Point3<f32>;
type Vector3 = nalgebra::Vector3<f32>;
type UnitQuaternion = nalgebra::UnitQuaternion<f32>;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Isometry3 {
    inner: nalgebra::Isometry3<f32>,
}

impl Isometry3 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::Isometry3::identity(),
        }
    }

    #[inline]
    pub fn from_parts(translation: Vector3, rotation: UnitQuaternion) -> Self {
        Self {
            inner: nalgebra::Isometry3::from_parts(translation.into(), rotation),
        }
    }

    #[inline]
    pub fn from_translation(translation: Vector3) -> Self {
        Self::from_parts(translation, UnitQuaternion::identity())
    }

    #[inline]
    pub fn from_rotation(rotation: UnitQuaternion) -> Self {
        Self::from_parts(Vector3::zeros(), rotation)
    }

    #[inline]
    pub fn from_rotated_translation(translation: Vector3, rotation: UnitQuaternion) -> Self {
        Self::from_parts(rotation.transform_vector(&translation), rotation)
    }

    #[inline]
    pub fn translated(&self, translation: &Vector3) -> Self {
        Self {
            inner: nalgebra::Translation3::from(*translation) * self.inner,
        }
    }

    #[inline]
    pub fn rotated(&self, rotation: &UnitQuaternion) -> Self {
        Self {
            inner: rotation * self.inner,
        }
    }

    #[inline]
    pub fn apply_to_translation(&self, translation: &Vector3) -> Self {
        Self {
            inner: self.inner * nalgebra::Translation3::from(*translation),
        }
    }

    #[inline]
    pub fn apply_to_rotation(&self, rotation: &UnitQuaternion) -> Self {
        Self {
            inner: self.inner * rotation,
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
    }

    #[inline]
    pub fn translation(&self) -> &Vector3 {
        &self.inner.translation.vector
    }

    #[inline]
    pub fn rotation(&self) -> &UnitQuaternion {
        &self.inner.rotation
    }

    #[inline]
    pub fn transform_point(&self, point: &Point3) -> Point3 {
        self.inner.transform_point(point)
    }

    #[inline]
    pub fn transform_vector(&self, vector: &Vector3) -> Vector3 {
        self.inner.transform_vector(vector)
    }

    #[inline]
    pub fn inverse_transform_point(&self, point: &Point3) -> Point3 {
        self.inner.inverse_transform_point(point)
    }

    #[inline]
    pub fn inverse_transform_vector(&self, vector: &Vector3) -> Vector3 {
        self.inner.inverse_transform_vector(vector)
    }

    pub fn _inner(&self) -> &nalgebra::Isometry3<f32> {
        &self.inner
    }
}

impl<'a> std::ops::Mul<&'a Isometry3> for &Isometry3 {
    type Output = Isometry3;

    #[inline]
    fn mul(self, rhs: &'a Isometry3) -> Self::Output {
        Isometry3 {
            inner: self.inner * rhs.inner,
        }
    }
}

impl std::ops::Mul<Isometry3> for &Isometry3 {
    type Output = Isometry3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: Isometry3) -> Self::Output {
        self * &rhs
    }
}

impl std::ops::Mul<Isometry3> for Isometry3 {
    type Output = Isometry3;

    #[inline]
    fn mul(self, rhs: Isometry3) -> Self::Output {
        &self * &rhs
    }
}

impl<'a> std::ops::Mul<&'a Isometry3> for Isometry3 {
    type Output = Isometry3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: &'a Isometry3) -> Self::Output {
        &self * rhs
    }
}

impl AbsDiffEq for Isometry3 {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.inner.abs_diff_eq(&other.inner, epsilon)
    }
}
