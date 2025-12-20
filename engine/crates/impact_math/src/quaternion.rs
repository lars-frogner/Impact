//! Quaternions.

use crate::matrix::{Matrix3, Matrix4};
use approx::{AbsDiffEq, RelativeEq};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;

type Point3 = nalgebra::Point3<f32>;
type Vector3 = nalgebra::Vector3<f32>;
type UnitVector3 = nalgebra::UnitVector3<f32>;
type Vector4 = nalgebra::Vector4<f32>;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Quaternion {
    inner: nalgebra::Quaternion<f32>,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct UnitQuaternion {
    inner: nalgebra::UnitQuaternion<f32>,
}

impl Quaternion {
    #[inline]
    pub const fn from_parts(real: f32, imag: Vector3) -> Self {
        Self {
            inner: nalgebra::Quaternion::from_vector(Vector4::new(
                imag.data.0[0][0],
                imag.data.0[0][1],
                imag.data.0[0][2],
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
        self.inner.imag()
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

impl AbsDiffEq for Quaternion {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.inner.abs_diff_eq(&other.inner, epsilon)
    }
}

impl RelativeEq for Quaternion {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.inner.relative_eq(&other.inner, epsilon, max_relative)
    }
}

impl UnitQuaternion {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::identity(),
        }
    }

    #[inline]
    pub fn new_normalize(quaternion: Quaternion) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::new_normalize(quaternion.inner),
        }
    }

    #[inline]
    pub const fn new_unchecked(quaternion: Quaternion) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::new_unchecked(quaternion.inner),
        }
    }

    #[inline]
    pub fn from_axis_angle(axis: &UnitVector3, angle: f32) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::from_axis_angle(axis, angle),
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
        nalgebra::UnitQuaternion::rotation_between_axis(a, b).map(|inner| Self { inner })
    }

    #[inline]
    pub fn look_at_rh(dir: &Vector3, up: &Vector3) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::look_at_rh(dir, up),
        }
    }

    #[inline]
    pub fn from_basis_unchecked(basis: &[Vector3; 3]) -> Self {
        Self {
            inner: nalgebra::UnitQuaternion::from_basis_unchecked(basis),
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
        self.inner.imag()
    }

    #[inline]
    pub fn axis_angle(&self) -> Option<(UnitVector3, f32)> {
        self.inner.axis_angle()
    }

    #[inline]
    pub fn axis(&self) -> Option<UnitVector3> {
        self.inner.axis()
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

    #[inline]
    pub fn rotate_unit_vector(&self, vector: &UnitVector3) -> UnitVector3 {
        self.inner * vector
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

impl AbsDiffEq for UnitQuaternion {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.inner.abs_diff_eq(&other.inner, epsilon)
    }
}

impl RelativeEq for UnitQuaternion {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.inner.relative_eq(&other.inner, epsilon, max_relative)
    }
}

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
impl_roc_for_library_provided_primitives! {
//  Type              Pkg   Parents  Module          Roc name        Postfix  Precision
    UnitQuaternion => core, None,    UnitQuaternion, UnitQuaternion, None,    PrecisionIrrelevant,
}
