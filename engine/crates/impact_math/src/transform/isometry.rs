//! Isometry transforms.

use crate::{point::Point3, quaternion::UnitQuaternion, vector::Vector3};
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
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
            inner: nalgebra::Isometry3::from_parts(
                (*translation._inner()).into(),
                *rotation._inner(),
            ),
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
    pub fn applied_to_translation(&self, translation: &Vector3) -> Self {
        Self {
            inner: self.inner * nalgebra::Translation3::from(*translation._inner()),
        }
    }

    #[inline]
    pub fn applied_to_rotation(&self, rotation: &UnitQuaternion) -> Self {
        Self {
            inner: self.inner * rotation._inner(),
        }
    }

    #[inline]
    pub fn inverted(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
    }

    #[inline]
    pub fn translation(&self) -> &Vector3 {
        bytemuck::from_bytes(bytemuck::bytes_of(&self.inner.translation.vector))
    }

    #[inline]
    pub fn rotation(&self) -> &UnitQuaternion {
        bytemuck::from_bytes(bytemuck::bytes_of(&self.inner.rotation))
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

    #[inline]
    pub fn _inner(&self) -> &nalgebra::Isometry3<f32> {
        &self.inner
    }
}

impl_binop!(Mul, mul, Isometry3, Isometry3, Isometry3, |a, b| {
    Isometry3 {
        inner: a.inner * b.inner,
    }
});

impl_abs_diff_eq!(Isometry3, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Isometry3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});
