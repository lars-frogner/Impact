//! Isometry transforms.

use crate::quaternion::UnitQuaternion;
use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix4;

type Point3 = nalgebra::Point3<f32>;
type Vector3 = nalgebra::Vector3<f32>;

#[repr(transparent)]
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
    pub fn from_matrix_unchecked(matrix: Matrix4<f32>) -> Self {
        Self {
            inner: nalgebra::Projective3::from_matrix_unchecked(matrix),
        }
    }

    #[inline]
    pub fn matrix(&self) -> &Matrix4<f32> {
        self.inner.matrix()
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
    pub fn inverse(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
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
