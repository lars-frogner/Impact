//! Similarity transforms.

use super::Isometry3;
use crate::quaternion::UnitQuaternion;
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};

type Point3 = nalgebra::Point3<f32>;
type Vector3 = nalgebra::Vector3<f32>;
type Matrix4 = nalgebra::Matrix4<f32>;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Similarity3 {
    inner: nalgebra::Similarity3<f32>,
}

impl Similarity3 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::Similarity3::identity(),
        }
    }

    #[inline]
    pub fn from_parts(translation: Vector3, rotation: UnitQuaternion, scaling: f32) -> Self {
        Self {
            inner: nalgebra::Similarity3::from_parts(
                translation.into(),
                *rotation._inner(),
                scaling,
            ),
        }
    }

    #[inline]
    pub fn from_isometry(isometry: Isometry3) -> Self {
        Self::from_parts(*isometry.translation(), *isometry.rotation(), 1.0)
    }

    #[inline]
    pub fn from_translation(translation: Vector3) -> Self {
        Self::from_parts(translation, UnitQuaternion::identity(), 1.0)
    }

    #[inline]
    pub fn from_rotation(rotation: UnitQuaternion) -> Self {
        Self::from_parts(Vector3::zeros(), rotation, 1.0)
    }

    #[inline]
    pub fn from_scaling(scaling: f32) -> Self {
        Self::from_parts(Vector3::zeros(), UnitQuaternion::identity(), scaling)
    }

    #[inline]
    pub fn from_scaled_translation(translation: Vector3, scaling: f32) -> Self {
        Self::from_parts(translation * scaling, UnitQuaternion::identity(), scaling)
    }

    #[inline]
    pub fn from_scaled_rotation(rotation: UnitQuaternion, scaling: f32) -> Self {
        Self::from_rotation(rotation).scaled(scaling)
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
            inner: rotation._inner() * self.inner,
        }
    }

    #[inline]
    pub fn scaled(&self, scaling: f32) -> Self {
        Self {
            inner: self.inner.append_scaling(scaling),
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
            inner: self.inner * rotation._inner(),
        }
    }

    #[inline]
    pub fn apply_to_scaling(&self, scaling: f32) -> Self {
        Self {
            inner: self.inner.prepend_scaling(scaling),
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
    }

    #[inline]
    pub fn to_matrix(&self) -> Matrix4 {
        self.inner.to_homogeneous()
    }

    #[inline]
    pub fn isometry(&self) -> &Isometry3 {
        bytemuck::from_bytes(bytemuck::bytes_of(&self.inner.isometry))
    }

    #[inline]
    pub fn translation(&self) -> &Vector3 {
        self.isometry().translation()
    }

    #[inline]
    pub fn rotation(&self) -> &UnitQuaternion {
        self.isometry().rotation()
    }

    #[inline]
    pub fn scaling(&self) -> f32 {
        self.inner.scaling()
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
}

impl<'a> std::ops::Mul<&'a Isometry3> for &Similarity3 {
    type Output = Similarity3;

    #[inline]
    fn mul(self, rhs: &'a Isometry3) -> Self::Output {
        Similarity3 {
            inner: self.inner * rhs._inner(),
        }
    }
}

impl std::ops::Mul<Isometry3> for &Similarity3 {
    type Output = Similarity3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: Isometry3) -> Self::Output {
        self * &rhs
    }
}

impl std::ops::Mul<Isometry3> for Similarity3 {
    type Output = Similarity3;

    #[inline]
    fn mul(self, rhs: Isometry3) -> Self::Output {
        &self * &rhs
    }
}

impl<'a> std::ops::Mul<&'a Isometry3> for Similarity3 {
    type Output = Similarity3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: &'a Isometry3) -> Self::Output {
        &self * rhs
    }
}

impl<'a> std::ops::Mul<&'a Similarity3> for &Isometry3 {
    type Output = Similarity3;

    #[inline]
    fn mul(self, rhs: &'a Similarity3) -> Self::Output {
        Similarity3 {
            inner: self._inner() * rhs.inner,
        }
    }
}

impl std::ops::Mul<Similarity3> for &Isometry3 {
    type Output = Similarity3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: Similarity3) -> Self::Output {
        self * &rhs
    }
}

impl std::ops::Mul<Similarity3> for Isometry3 {
    type Output = Similarity3;

    #[inline]
    fn mul(self, rhs: Similarity3) -> Self::Output {
        &self * &rhs
    }
}

impl<'a> std::ops::Mul<&'a Similarity3> for Isometry3 {
    type Output = Similarity3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: &'a Similarity3) -> Self::Output {
        &self * rhs
    }
}

impl<'a> std::ops::Mul<&'a Similarity3> for &Similarity3 {
    type Output = Similarity3;

    #[inline]
    fn mul(self, rhs: &'a Similarity3) -> Self::Output {
        Similarity3 {
            inner: self.inner * rhs.inner,
        }
    }
}

impl std::ops::Mul<Similarity3> for &Similarity3 {
    type Output = Similarity3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: Similarity3) -> Self::Output {
        self * &rhs
    }
}

impl std::ops::Mul<Similarity3> for Similarity3 {
    type Output = Similarity3;

    #[inline]
    fn mul(self, rhs: Similarity3) -> Self::Output {
        &self * &rhs
    }
}

impl<'a> std::ops::Mul<&'a Similarity3> for Similarity3 {
    type Output = Similarity3;

    #[allow(clippy::op_ref)]
    #[inline]
    fn mul(self, rhs: &'a Similarity3) -> Self::Output {
        &self * rhs
    }
}

impl AbsDiffEq for Similarity3 {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.inner.abs_diff_eq(&other.inner, epsilon)
    }
}
