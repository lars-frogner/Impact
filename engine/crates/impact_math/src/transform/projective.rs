//! Isometry transforms.

use crate::{matrix::Matrix4, point::Point3, quaternion::UnitQuaternion, vector::Vector3};
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Projective3 {
    inner: nalgebra::Projective3<f32>,
}

impl Projective3 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::Projective3::identity(),
        }
    }

    #[inline]
    pub fn from_matrix_unchecked(matrix: Matrix4) -> Self {
        Self {
            inner: nalgebra::Projective3::from_matrix_unchecked(*matrix._inner()),
        }
    }

    #[inline]
    pub fn matrix(&self) -> &Matrix4 {
        bytemuck::from_bytes(bytemuck::bytes_of(self.inner.matrix()))
    }

    #[inline]
    pub fn translated(&self, translation: &Vector3) -> Self {
        Self {
            inner: nalgebra::Translation3::from(*translation._inner()) * self.inner,
        }
    }

    #[inline]
    pub fn rotated(&self, rotation: &UnitQuaternion) -> Self {
        Self {
            inner: rotation._inner() * self.inner,
        }
    }

    #[inline]
    pub fn apply_to_translation(&self, translation: &Vector3) -> Self {
        Self {
            inner: self.inner * nalgebra::Translation3::from(*translation._inner()),
        }
    }

    #[inline]
    pub fn apply_to_rotation(&self, rotation: &UnitQuaternion) -> Self {
        Self {
            inner: self.inner * rotation._inner(),
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
    }

    #[inline]
    pub fn transform_point(&self, point: &Point3) -> Point3 {
        Point3::_wrap(self.inner.transform_point(point._inner()))
    }

    #[inline]
    pub fn transform_vector(&self, vector: &Vector3) -> Vector3 {
        Vector3::_wrap(self.inner.transform_vector(vector._inner()))
    }

    #[inline]
    pub fn inverse_transform_point(&self, point: &Point3) -> Point3 {
        Point3::_wrap(self.inner.inverse_transform_point(point._inner()))
    }

    #[inline]
    pub fn inverse_transform_vector(&self, vector: &Vector3) -> Vector3 {
        Vector3::_wrap(self.inner.inverse_transform_vector(vector._inner()))
    }
}

impl_abs_diff_eq!(Projective3, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Projective3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});
