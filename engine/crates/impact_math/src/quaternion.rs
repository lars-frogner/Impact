//! Quaternions.

use crate::{
    matrix::{Matrix3, Matrix4},
    point::Point3,
    vector::{UnitVector3, Vector3},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Quaternion {
    inner: nalgebra::Quaternion<f32>,
}

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct UnitQuaternion {
    inner: nalgebra::UnitQuaternion<f32>,
}

impl Quaternion {
    #[inline]
    pub const fn from_parts(real: f32, imag: Vector3) -> Self {
        Self {
            inner: nalgebra::Quaternion::from_vector(nalgebra::Vector4::new(
                imag._inner().data.0[0][0],
                imag._inner().data.0[0][1],
                imag._inner().data.0[0][2],
                real,
            )),
        }
    }

    #[inline]
    pub const fn from_imag(imag: Vector3) -> Self {
        Self::from_parts(0.0, imag)
    }

    #[inline]
    pub fn real(&self) -> f32 {
        self.inner.w
    }

    #[inline]
    pub fn imag(&self) -> Vector3 {
        Vector3::_wrap(self.inner.imag())
    }

    #[inline]
    pub fn negated(&self) -> Self {
        use std::ops::Neg;
        Self {
            inner: self.inner.neg(),
        }
    }
}

impl_binop!(Add, add, Quaternion, Quaternion, Quaternion, |a, b| {
    Quaternion {
        inner: a.inner + b.inner,
    }
});

impl_binop!(Mul, mul, Quaternion, Quaternion, Quaternion, |a, b| {
    Quaternion {
        inner: a.inner * b.inner,
    }
});

impl_abs_diff_eq!(Quaternion, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Quaternion, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl UnitQuaternion {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::identity(),
        }
    }

    #[inline]
    pub fn normalized_from(quaternion: Quaternion) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::new_normalize(quaternion.inner),
        }
    }

    #[inline]
    pub const fn unchecked_from(quaternion: Quaternion) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::new_unchecked(quaternion.inner),
        }
    }

    #[inline]
    pub fn from_axis_angle(axis: &UnitVector3, angle: f32) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::from_axis_angle(axis._inner(), angle),
        }
    }

    #[inline]
    pub fn from_euler_angles(roll: f32, pitch: f32, yaw: f32) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::from_euler_angles(roll, pitch, yaw),
        }
    }

    #[inline]
    pub fn rotation_between_axis(a: &UnitVector3, b: &UnitVector3) -> Option<Self> {
        nalgebra::UnitQuaternion::rotation_between_axis(a._inner(), b._inner())
            .map(|inner| Self { inner })
    }

    #[inline]
    pub fn look_at_rh(dir: &Vector3, up: &Vector3) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::look_at_rh(dir._inner(), up._inner()),
        }
    }

    #[inline]
    pub fn from_basis_unchecked(basis: &[Vector3; 3]) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::from_basis_unchecked(&basis.map(|v| *v._inner())),
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
    }

    #[inline]
    pub fn negated(&self) -> Self {
        use std::ops::Neg;
        Self {
            inner: nalgebra::UnitQuaternion::new_unchecked(self.inner.neg()),
        }
    }

    #[inline]
    pub fn real(&self) -> f32 {
        self.inner.w
    }

    #[inline]
    pub fn imag(&self) -> Vector3 {
        Vector3::_wrap(self.inner.imag())
    }

    #[inline]
    pub fn axis_angle(&self) -> Option<(UnitVector3, f32)> {
        self.inner
            .axis_angle()
            .map(|(axis, angle)| (UnitVector3::_wrap(axis), angle))
    }

    #[inline]
    pub fn axis(&self) -> Option<UnitVector3> {
        self.inner.axis().map(UnitVector3::_wrap)
    }

    #[inline]
    pub fn angle(&self) -> f32 {
        self.inner.angle()
    }

    #[inline]
    pub fn euler_angles(&self) -> (f32, f32, f32) {
        self.inner.euler_angles()
    }

    #[inline]
    pub fn to_quaternion(&self) -> Quaternion {
        Quaternion {
            inner: self.inner.into_inner(),
        }
    }

    #[inline]
    pub fn to_rotation_matrix(&self) -> Matrix3 {
        Matrix3::_wrap(*self.inner.to_rotation_matrix().matrix())
    }

    #[inline]
    pub fn to_homogeneous_matrix(&self) -> Matrix4 {
        Matrix4::_wrap(self.inner.to_homogeneous())
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
    pub fn rotate_unit_vector(&self, vector: &UnitVector3) -> UnitVector3 {
        UnitVector3::_wrap(self.inner * vector._inner())
    }

    #[inline]
    pub fn _inner(&self) -> &nalgebra::UnitQuaternion<f32> {
        &self.inner
    }
}

impl_binop!(
    Mul,
    mul,
    UnitQuaternion,
    UnitQuaternion,
    UnitQuaternion,
    |a, b| {
        UnitQuaternion {
            inner: a.inner * b.inner,
        }
    }
);

impl_abs_diff_eq!(UnitQuaternion, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(UnitQuaternion, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
impl_roc_for_library_provided_primitives! {
//  Type              Pkg   Parents  Module          Roc name        Postfix  Precision
    UnitQuaternion => core, None,    UnitQuaternion, UnitQuaternion, None,    PrecisionIrrelevant,
}
