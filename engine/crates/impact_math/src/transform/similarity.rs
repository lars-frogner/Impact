//! Similarity transforms.

use super::Isometry3;
use crate::{matrix::Matrix4, point::Point3, quaternion::UnitQuaternion, vector::Vector3};
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
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
                (*translation._inner()).into(),
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
    pub fn scaled(&self, scaling: f32) -> Self {
        Self {
            inner: self.inner.append_scaling(scaling),
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
    pub fn applied_to_scaling(&self, scaling: f32) -> Self {
        Self {
            inner: self.inner.prepend_scaling(scaling),
        }
    }

    #[inline]
    pub fn inverted(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
    }

    #[inline]
    pub fn to_matrix(&self) -> Matrix4 {
        Matrix4::_wrap(self.inner.to_homogeneous())
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

impl_binop!(Mul, mul, Similarity3, Isometry3, Similarity3, |a, b| {
    Similarity3 {
        inner: a.inner * b._inner(),
    }
});

impl_binop!(Mul, mul, Isometry3, Similarity3, Similarity3, |a, b| {
    Similarity3 {
        inner: a._inner() * b.inner,
    }
});

impl_binop!(Mul, mul, Similarity3, Similarity3, Similarity3, |a, b| {
    Similarity3 {
        inner: a.inner * b.inner,
    }
});

impl_abs_diff_eq!(Similarity3, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Similarity3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});
