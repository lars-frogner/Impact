//! Quaternions.

use crate::{
    matrix::{Matrix3A, Matrix4A},
    point::Point3A,
    vector::{UnitVector3A, Vector3, Vector3A, Vector4, Vector4A},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;
use std::{fmt, ops::Mul};

/// A quaternion.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`QuaternionA`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 4]", from = "[f32; 4]")
)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct Quaternion {
    imag: Vector3,
    real: f32,
}

/// A quaternion aligned to 16 bytes.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned
/// [`Quaternion`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
pub struct QuaternionA {
    inner: glam::Quat,
}

/// A quaternion of unit length, representing a rotation.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`UnitQuaternionA`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 4]", from = "[f32; 4]")
)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct UnitQuaternion {
    imag: Vector3,
    real: f32,
}

/// A quaternion of unit length, representing a rotation, aligned to 16 bytes.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned
/// [`UnitQuaternion`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
pub struct UnitQuaternionA {
    inner: glam::Quat,
}

impl Quaternion {
    /// Creates a quaternion with the given imaginary and real parts.
    #[inline]
    pub const fn from_parts(imag: Vector3, real: f32) -> Self {
        Self { imag, real }
    }

    /// Creates a quaternion from the given vector, with the last component
    /// representing the real part.
    #[inline]
    pub const fn from_vector(vector: Vector4) -> Self {
        Self {
            imag: vector.xyz(),
            real: vector.w(),
        }
    }

    /// Creates a quaternion with the given imaginary part and zero real part.
    #[inline]
    pub const fn from_imag(imag: Vector3) -> Self {
        Self::from_parts(imag, 0.0)
    }

    /// Creates a quaternion with the given real part and zero imaginary part.
    #[inline]
    pub const fn from_real(real: f32) -> Self {
        Self::from_parts(Vector3::zeros(), real)
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub fn imag(&self) -> &Vector3 {
        &self.imag
    }

    /// The real part of the quaternion.
    #[inline]
    pub fn real(&self) -> f32 {
        self.real
    }

    /// Converts the quaternion to the 16-byte aligned SIMD-friendly
    /// [`QuaternionA`].
    #[inline]
    pub fn aligned(&self) -> QuaternionA {
        QuaternionA::from_parts(self.imag().aligned(), self.real())
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::from_real(1.0)
    }
}

impl_abs_diff_eq!(Quaternion, |a, b, epsilon| {
    a.imag.abs_diff_eq(&b.imag, epsilon) && a.real.abs_diff_eq(&b.real, epsilon)
});

impl_relative_eq!(Quaternion, |a, b, epsilon, max_relative| {
    a.imag.relative_eq(&b.imag, epsilon, max_relative)
        && a.real.relative_eq(&b.real, epsilon, max_relative)
});

impl QuaternionA {
    /// Creates a quaternion with the given imaginary and real parts.
    #[inline]
    pub fn from_parts(imag: Vector3A, real: f32) -> Self {
        Self::wrap(glam::Quat::from_xyzw(imag.x(), imag.y(), imag.z(), real))
    }

    /// Creates a quaternion from the given vector, with the last component
    /// representing the real part.
    #[inline]
    pub const fn from_vector(vector: Vector4A) -> Self {
        Self::wrap(glam::Quat::from_vec4(vector.unwrap()))
    }

    /// Creates a quaternion with the given imaginary part and zero real part.
    #[inline]
    pub fn from_imag(imag: Vector3A) -> Self {
        Self::wrap(glam::Quat::from_xyzw(imag.x(), imag.y(), imag.z(), 0.0))
    }

    /// Creates a quaternion with the given real part and zero imaginary part.
    #[inline]
    pub const fn from_real(real: f32) -> Self {
        Self::wrap(glam::Quat::from_xyzw(0.0, 0.0, 0.0, real))
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub fn imag(&self) -> Vector3A {
        Vector3A::wrap(self.inner.xyz().to_vec3a())
    }

    /// The real part of the quaternion.
    #[inline]
    pub fn real(&self) -> f32 {
        self.inner.w
    }

    /// Converts the quaternion to the 4-byte aligned cache-friendly
    /// [`Quaternion`].
    #[inline]
    pub fn unaligned(&self) -> Quaternion {
        Quaternion::from_parts(self.imag().unaligned(), self.real())
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Quat) -> Self {
        Self { inner }
    }

    #[inline]
    pub(crate) const fn unwrap(self) -> glam::Quat {
        self.inner
    }
}

impl Default for QuaternionA {
    fn default() -> Self {
        Self::from_real(1.0)
    }
}

impl_binop!(Add, add, QuaternionA, QuaternionA, QuaternionA, |a, b| {
    QuaternionA::wrap(a.inner.add(b.inner))
});

impl_binop!(Sub, sub, QuaternionA, QuaternionA, QuaternionA, |a, b| {
    QuaternionA::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, QuaternionA, QuaternionA, QuaternionA, |a, b| {
    QuaternionA::wrap(a.inner.mul_quat(b.inner))
});

impl_binop!(Mul, mul, QuaternionA, f32, QuaternionA, |a, b| {
    QuaternionA::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, QuaternionA, QuaternionA, |a, b| {
    b.mul(*a)
});

impl_binop!(Div, div, QuaternionA, f32, QuaternionA, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, QuaternionA, QuaternionA, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, QuaternionA, QuaternionA, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, QuaternionA, QuaternionA, |a, b| {
    a.inner.mul_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, QuaternionA, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, QuaternionA, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, QuaternionA, QuaternionA, |val| {
    QuaternionA::wrap(val.inner.neg())
});

impl_abs_diff_eq!(QuaternionA, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(QuaternionA, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl UnitQuaternion {
    /// Creates a unit quaternion representing the identity rotation.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            imag: Vector3::zeros(),
            real: 1.0,
        }
    }

    /// Converts the given quaternion to a unit quaternion, assuming it is
    /// already normalized.
    #[inline]
    pub const fn unchecked_from(quaternion: Quaternion) -> Self {
        Self {
            imag: quaternion.imag,
            real: quaternion.real,
        }
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub const fn imag(&self) -> &Vector3 {
        &self.imag
    }

    /// The real part of the quaternion.
    #[inline]
    pub const fn real(&self) -> f32 {
        self.real
    }

    /// This unit quaternion as a [`Quaternion`].
    #[inline]
    pub fn as_quaternion(&self) -> &Quaternion {
        bytemuck::cast_ref(self)
    }

    /// Converts the quaternion to the 16-byte aligned SIMD-friendly
    /// [`UnitQuaternionA`].
    #[inline]
    pub fn aligned(&self) -> UnitQuaternionA {
        UnitQuaternionA::unchecked_from(QuaternionA::from_parts(self.imag().aligned(), self.real()))
    }
}

impl Default for UnitQuaternion {
    fn default() -> Self {
        Self::identity()
    }
}

impl_abs_diff_eq!(UnitQuaternion, |a, b, epsilon| {
    a.imag.abs_diff_eq(&b.imag, epsilon) && a.real.abs_diff_eq(&b.real, epsilon)
});

impl_relative_eq!(UnitQuaternion, |a, b, epsilon, max_relative| {
    a.imag.relative_eq(&b.imag, epsilon, max_relative)
        && a.real.relative_eq(&b.real, epsilon, max_relative)
});

impl UnitQuaternionA {
    /// Creates a unit quaternion representing the identity rotation.
    #[inline]
    pub const fn identity() -> Self {
        Self::wrap(glam::Quat::IDENTITY)
    }

    /// Converts the given quaternion to a unit quaternion, assuming it is
    /// already normalized.
    #[inline]
    pub const fn unchecked_from(quaternion: QuaternionA) -> Self {
        Self::wrap(quaternion.unwrap())
    }

    /// Creates a unit quaternion by normalizing the given quaternion. If the
    /// quaternion has zero length, the result will be non-finite.
    #[inline]
    pub fn normalized_from(quaternion: QuaternionA) -> Self {
        Self::wrap(quaternion.unwrap().normalize())
    }

    /// Creates a unit quaternion representing a rotation of the given angle (in
    /// radians) about the given axis.
    #[inline]
    pub fn from_axis_angle(axis: &UnitVector3A, angle: f32) -> Self {
        Self::wrap(glam::Quat::from_axis_angle(axis.unwrap().to_vec3(), angle))
    }

    /// Creates a unit quaternion representing the rotation of the given Euler
    /// angles (in radians) about the x- (`roll`), y- (`pitch`) and z-axis
    /// (`yaw`), in that order.
    #[inline]
    pub fn from_euler_angles(roll: f32, pitch: f32, yaw: f32) -> Self {
        Self::wrap(glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            roll,
            pitch,
            yaw,
        ))
    }

    /// Creates a unit quaternion representing the smallest rotation from one
    /// direction to another.
    #[inline]
    pub fn rotation_between_axes(from: &UnitVector3A, to: &UnitVector3A) -> Self {
        Self::wrap(glam::Quat::from_rotation_arc(
            from.unwrap().to_vec3(),
            to.unwrap().to_vec3(),
        ))
    }

    /// Creates a unit quaternion reprenting the rotation aligning the positive
    /// z-axis with the given view direction and the positive y-axis with the
    /// given up direction.
    #[inline]
    pub fn look_to_rh(dir: &UnitVector3A, up: &UnitVector3A) -> Self {
        Self::wrap(glam::Quat::look_to_rh(
            dir.unwrap().to_vec3(),
            up.unwrap().to_vec3(),
        ))
    }

    /// Creates a unit quaternion representing the orientation of the reference
    /// frame with the given three basis vectors. The vectors are assumed
    /// normalized and perpendicular.
    #[inline]
    pub fn from_basis_unchecked(basis: &[Vector3A; 3]) -> Self {
        // `glam::Mat3A` is column-major, so we can just cast the reference
        Self::wrap(glam::Quat::from_mat3a(bytemuck::cast_ref(basis)))
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub fn imag(&self) -> Vector3A {
        Vector3A::wrap(self.inner.xyz().to_vec3a())
    }

    /// The real part of the quaternion.
    #[inline]
    pub fn real(&self) -> f32 {
        self.inner.w
    }

    /// Computes the unit quaternion representing the reverse of this
    /// quaternion's rotation.
    #[inline]
    pub fn inverse(&self) -> Self {
        Self::wrap(self.inner.conjugate())
    }

    /// Computes the axis and angle of this rotation.
    #[inline]
    pub fn axis_angle(&self) -> (UnitVector3A, f32) {
        let (axis, angle) = self.inner.to_axis_angle();
        (UnitVector3A::wrap(axis.to_vec3a()), angle)
    }

    /// Computes the axis of this rotation.
    #[inline]
    pub fn axis(&self) -> UnitVector3A {
        self.axis_angle().0
    }

    /// Computes the angle of this rotation.
    #[inline]
    pub fn angle(&self) -> f32 {
        self.axis_angle().1
    }

    /// Computes the Euler angles of this rotation.
    #[inline]
    pub fn euler_angles(&self) -> (f32, f32, f32) {
        self.inner.to_euler(glam::EulerRot::XYZ)
    }

    /// Converts the quaternion to a 3x3 rotation matrix.
    #[inline]
    pub fn to_rotation_matrix(&self) -> Matrix3A {
        Matrix3A::wrap(glam::Mat3A::from_quat(self.inner))
    }

    /// Converts the quaternion to a 4x4 homogeneous matrix.
    #[inline]
    pub fn to_homogeneous_matrix(&self) -> Matrix4A {
        Matrix4A::wrap(glam::Mat4::from_quat(self.inner))
    }

    /// Applies the rotation to the given point.
    #[inline]
    pub fn rotate_point(&self, point: &Point3A) -> Point3A {
        Point3A::wrap(self.inner.mul_vec3a(point.unwrap()))
    }

    /// Applies the rotation to the given vector.
    #[inline]
    pub fn rotate_vector(&self, vector: &Vector3A) -> Vector3A {
        Vector3A::wrap(self.inner.mul_vec3a(vector.unwrap()))
    }

    /// Applies the rotation to the given unit vector.
    #[inline]
    pub fn rotate_unit_vector(&self, vector: &UnitVector3A) -> UnitVector3A {
        UnitVector3A::wrap(self.inner.mul_vec3a(vector.unwrap()))
    }

    /// This unit quaternion as a [`QuaternionA`].
    #[inline]
    pub fn as_quaternion(&self) -> &QuaternionA {
        bytemuck::cast_ref(self)
    }

    /// Converts the quaternion to the 4-byte aligned cache-friendly
    /// [`UnitQuaternion`].
    #[inline]
    pub fn unaligned(&self) -> UnitQuaternion {
        UnitQuaternion::unchecked_from(Quaternion::from_parts(
            Vector3::from_glam(self.inner.xyz()),
            self.real(),
        ))
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Quat) -> Self {
        Self { inner }
    }
}

impl Default for UnitQuaternionA {
    fn default() -> Self {
        Self::identity()
    }
}

impl_binop!(
    Mul,
    mul,
    UnitQuaternionA,
    UnitQuaternionA,
    UnitQuaternionA,
    |a, b| { UnitQuaternionA::wrap(a.inner.mul_quat(b.inner)) }
);

impl_binop!(Mul, mul, UnitQuaternionA, f32, QuaternionA, |a, b| {
    QuaternionA::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, UnitQuaternionA, QuaternionA, |a, b| {
    b.mul(*a)
});

impl_binop!(Div, div, UnitQuaternionA, f32, QuaternionA, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(
    MulAssign,
    mul_assign,
    UnitQuaternionA,
    UnitQuaternionA,
    |a, b| {
        a.inner.mul_assign(b.inner);
    }
);

impl_unary_op!(Neg, neg, UnitQuaternionA, UnitQuaternionA, |val| {
    UnitQuaternionA::wrap(val.inner.neg())
});

impl_abs_diff_eq!(UnitQuaternionA, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(UnitQuaternionA, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl From<Quaternion> for [f32; 4] {
    fn from(q: Quaternion) -> [f32; 4] {
        [q.imag.x(), q.imag.y(), q.imag.z(), q.real]
    }
}

impl From<[f32; 4]> for Quaternion {
    fn from(arr: [f32; 4]) -> Quaternion {
        Quaternion::from_parts(Vector3::new(arr[0], arr[1], arr[2]), arr[3])
    }
}

impl From<UnitQuaternion> for [f32; 4] {
    fn from(q: UnitQuaternion) -> [f32; 4] {
        [q.imag.x(), q.imag.y(), q.imag.z(), q.real]
    }
}

impl From<[f32; 4]> for UnitQuaternion {
    fn from(arr: [f32; 4]) -> UnitQuaternion {
        UnitQuaternion::unchecked_from(Quaternion::from_parts(
            Vector3::new(arr[0], arr[1], arr[2]),
            arr[3],
        ))
    }
}

impl fmt::Debug for QuaternionA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuaternionA")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .field("w", &self.inner.w)
            .finish()
    }
}

impl fmt::Debug for UnitQuaternionA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitQuaternionA")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .field("w", &self.inner.w)
            .finish()
    }
}

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
impl_roc_for_library_provided_primitives! {
//  Type              Pkg   Parents  Module          Roc name        Postfix  Precision
    UnitQuaternion => core, None,    UnitQuaternion, UnitQuaternion, None,    PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use approx::assert_abs_diff_eq;
    use std::f32::consts::PI;

    // Test constants
    const EPSILON: f32 = 1e-6;

    // Quaternion tests
    #[test]
    fn quaternion_from_parts_works() {
        let real = 1.0;
        let imag = Vector3A::new(2.0, 3.0, 4.0);
        let quat = QuaternionA::from_parts(imag, real);

        assert_eq!(quat.real(), 1.0);
        assert_eq!(quat.imag(), imag);
    }

    #[test]
    fn quaternion_from_imag_works() {
        let imag = Vector3A::new(2.0, 3.0, 4.0);
        let quat = QuaternionA::from_imag(imag);

        assert_eq!(quat.real(), 0.0);
        assert_eq!(quat.imag(), imag);
    }

    #[test]
    fn quaternion_real_and_imag_accessors_work() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 5.0);

        assert_eq!(quat.real(), 5.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 1.0);
        assert_eq!(imag.y(), 2.0);
        assert_eq!(imag.z(), 3.0);
    }

    #[test]
    fn quaternion_neg_works() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, -2.0, 3.0), 2.0);
        let negated = -quat;

        assert_eq!(negated.real(), -2.0);
        let neg_imag = negated.imag();
        assert_eq!(neg_imag.x(), -1.0);
        assert_eq!(neg_imag.y(), 2.0);
        assert_eq!(neg_imag.z(), -3.0);
    }

    #[test]
    fn quaternion_addition_works() {
        let q1 = QuaternionA::from_parts(Vector3A::new(2.0, 3.0, 4.0), 1.0);
        let q2 = QuaternionA::from_parts(Vector3A::new(1.0, 1.0, 1.0), 2.0);

        let result = &q1 + &q2;
        assert_eq!(result.real(), 3.0);
        let result_imag = result.imag();
        assert_eq!(result_imag.x(), 3.0);
        assert_eq!(result_imag.y(), 4.0);
        assert_eq!(result_imag.z(), 5.0);
    }

    #[test]
    fn quaternion_multiplication_works() {
        let q1 = QuaternionA::from_parts(Vector3A::new(0.0, 0.0, 0.0), 1.0);
        let q2 = QuaternionA::from_parts(Vector3A::new(1.0, 0.0, 0.0), 0.0);

        let result = &q1 * &q2;
        // i * 1 = i, so real = 0, imag = (1, 0, 0)
        assert_abs_diff_eq!(result.real(), 0.0, epsilon = EPSILON);
        let result_imag = result.imag();
        assert_abs_diff_eq!(result_imag.x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result_imag.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result_imag.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn quaternion_default_works() {
        let quat = QuaternionA::default();
        assert_eq!(quat.real(), 1.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    // UnitQuaternion tests
    #[test]
    fn unit_quaternion_identity_works() {
        let identity = UnitQuaternionA::identity();
        assert_eq!(identity.real(), 1.0);
        let imag = identity.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn unit_quaternion_normalized_from_works() {
        let quat = QuaternionA::from_parts(Vector3A::new(0.0, 0.0, 0.0), 2.0);
        let unit = UnitQuaternionA::normalized_from(quat);

        // Should normalize to (1, 0, 0, 0)
        assert_abs_diff_eq!(unit.real(), 1.0, epsilon = EPSILON);
        let imag = unit.imag();
        assert_abs_diff_eq!(imag.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_unchecked_from_works() {
        let quat = QuaternionA::from_parts(Vector3A::new(0.0, 0.0, 0.0), 1.0);
        let unit = UnitQuaternionA::unchecked_from(quat);

        assert_eq!(unit.real(), 1.0);
        let imag = unit.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_works() {
        let axis = UnitVector3A::unit_z();
        let angle = PI / 2.0; // 90 degrees
        let unit = UnitQuaternionA::from_axis_angle(&axis, angle);

        // Rotation around Z axis by 90 degrees
        let (extracted_axis, extracted_angle) = unit.axis_angle();
        assert_abs_diff_eq!(extracted_angle, angle, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_axis.z(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_from_euler_angles_works() {
        let roll = 0.0;
        let pitch = 0.0;
        let yaw = PI / 2.0;
        let unit = UnitQuaternionA::from_euler_angles(roll, pitch, yaw);

        let (extracted_roll, extracted_pitch, extracted_yaw) = unit.euler_angles();
        assert_abs_diff_eq!(extracted_roll, roll, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_pitch, pitch, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_yaw, yaw, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_axes_works() {
        let axis_x = UnitVector3A::unit_x();
        let axis_y = UnitVector3A::unit_y();

        let rotation = UnitQuaternionA::rotation_between_axes(&axis_x, &axis_y);

        // Should rotate X axis to Y axis
        let rotated = rotation.rotate_unit_vector(&axis_x);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_look_to_rh_works() {
        let dir = UnitVector3A::neg_unit_z(); // Looking down negative Z
        let up = UnitVector3A::unit_y(); // Y is up

        let look_at = UnitQuaternionA::look_to_rh(&dir, &up);

        // Should be close to identity for this standard orientation
        assert_abs_diff_eq!(look_at.real().abs(), 1.0, epsilon = 0.1);
    }

    #[test]
    fn unit_quaternion_from_basis_unchecked_works() {
        let basis = [
            Vector3A::new(1.0, 0.0, 0.0), // X axis
            Vector3A::new(0.0, 1.0, 0.0), // Y axis
            Vector3A::new(0.0, 0.0, 1.0), // Z axis
        ];

        let quat = UnitQuaternionA::from_basis_unchecked(&basis);

        // Should be close to identity for standard basis
        assert_abs_diff_eq!(quat.real(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_inverse_works() {
        let axis = UnitVector3A::unit_z();
        let angle = PI / 4.0;
        let quat = UnitQuaternionA::from_axis_angle(&axis, angle);
        let inverse = quat.inverse();

        // q * q^-1 should be identity
        let product = &quat * &inverse;
        assert_abs_diff_eq!(product.real(), 1.0, epsilon = EPSILON);
        let imag = product.imag();
        assert_abs_diff_eq!(imag.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_negated_works() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_x(), PI / 4.0);
        let negated = -quat;

        // Negated quaternion represents the same rotation
        let vector = Vector3A::new(0.0, 1.0, 0.0);
        let rotated1 = quat.rotate_vector(&vector);
        let rotated2 = negated.rotate_vector(&vector);

        assert_abs_diff_eq!(rotated1.x(), rotated2.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated1.y(), rotated2.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated1.z(), rotated2.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_real_and_imag_works() {
        let axis = UnitVector3A::unit_z();
        let angle = PI / 2.0;
        let quat = UnitQuaternionA::from_axis_angle(&axis, angle);

        let real = quat.real();
        let imag = quat.imag();

        // For rotation around Z by PI/2: q = cos(PI/4) + sin(PI/4) * k
        assert_abs_diff_eq!(real, (PI / 4.0).cos(), epsilon = EPSILON);
        assert_abs_diff_eq!(imag.z(), (PI / 4.0).sin(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_axis_angle_extraction_works() {
        let original_axis = UnitVector3A::unit_y();
        let original_angle = PI / 3.0;
        let quat = UnitQuaternionA::from_axis_angle(&original_axis, original_angle);

        let (extracted_axis, extracted_angle) = quat.axis_angle();

        assert_abs_diff_eq!(extracted_angle, original_angle, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_axis.y(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_axis_extraction_works() {
        let original_axis = UnitVector3A::unit_x();
        let quat = UnitQuaternionA::from_axis_angle(&original_axis, PI / 4.0);

        let extracted_axis = quat.axis();
        assert_abs_diff_eq!(extracted_axis.x(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_angle_extraction_works() {
        let original_angle = PI / 6.0;
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), original_angle);

        let extracted_angle = quat.angle();
        assert_abs_diff_eq!(extracted_angle, original_angle, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_euler_angles_roundtrip_works() {
        let roll = 0.1;
        let pitch = 0.2;
        let yaw = 0.3;

        let quat = UnitQuaternionA::from_euler_angles(roll, pitch, yaw);
        let (extracted_roll, extracted_pitch, extracted_yaw) = quat.euler_angles();

        assert_abs_diff_eq!(extracted_roll, roll, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_pitch, pitch, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_yaw, yaw, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_quaternion_works() {
        let unit = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 4.0);
        let quat = unit.as_quaternion();

        assert_abs_diff_eq!(quat.real(), unit.real(), epsilon = EPSILON);
        let unit_imag = unit.imag();
        let quat_imag = quat.imag();
        assert_abs_diff_eq!(quat_imag.x(), unit_imag.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_imag.y(), unit_imag.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_imag.z(), unit_imag.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_rotation_matrix_works() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 2.0);
        let matrix = quat.to_rotation_matrix();

        // 90 degree rotation around Z should map X to Y
        let x_axis = Vector3A::new(1.0, 0.0, 0.0);
        let rotated = &matrix * &x_axis;

        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_homogeneous_matrix_works() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 2.0);
        let matrix = quat.to_homogeneous_matrix();

        // Should be a 4x4 matrix with rotation in upper-left 3x3 and no translation
        assert_abs_diff_eq!(matrix.element(0, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(1, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(2, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(3, 3), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotate_point_works() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 2.0);
        let point = Point3A::new(1.0, 0.0, 0.0);

        let rotated = quat.rotate_point(&point);

        // 90 degree rotation around Z maps (1,0,0) to (0,1,0)
        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotate_vector_works() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 2.0);
        let vector = Vector3A::new(1.0, 0.0, 0.0);

        let rotated = quat.rotate_vector(&vector);

        // 90 degree rotation around Z maps (1,0,0) to (0,1,0)
        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotate_unit_vector_works() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 2.0);
        let unit_vector = UnitVector3A::unit_x();

        let rotated = quat.rotate_unit_vector(&unit_vector);

        // 90 degree rotation around Z maps X to Y
        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_multiplication_works() {
        let q1 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 4.0);
        let q2 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 4.0);

        let result = &q1 * &q2;

        // Two 45-degree rotations should equal one 90-degree rotation
        let expected = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 2.0);

        assert_abs_diff_eq!(result.angle(), expected.angle(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_default_works() {
        let quat = UnitQuaternionA::default();

        // Default should be identity
        assert_eq!(quat.real(), 1.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    // Edge case tests
    #[test]
    fn quaternion_scalar_multiplication_by_zero() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat * 0.0;

        assert_eq!(result.real(), 0.0);
        let imag = result.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn quaternion_scalar_multiplication_by_one() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat * 1.0;

        assert_eq!(result.real(), quat.real());
        let result_imag = result.imag();
        let quat_imag = quat.imag();
        assert_eq!(result_imag.x(), quat_imag.x());
        assert_eq!(result_imag.y(), quat_imag.y());
        assert_eq!(result_imag.z(), quat_imag.z());
    }

    #[test]
    fn quaternion_scalar_multiplication_by_negative() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat * -1.0;

        assert_eq!(result.real(), -4.0);
        let imag = result.imag();
        assert_eq!(imag.x(), -1.0);
        assert_eq!(imag.y(), -2.0);
        assert_eq!(imag.z(), -3.0);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_with_negative_angle() {
        let axis = UnitVector3A::unit_z();
        let angle = -PI / 4.0;
        let quat = UnitQuaternionA::from_axis_angle(&axis, angle);

        // Negative angle should rotate in opposite direction
        let vector = Vector3A::new(1.0, 0.0, 0.0);
        let rotated = quat.rotate_vector(&vector);

        // -45 degrees around Z should have positive x and negative y components
        assert!(rotated.x() > 0.0);
        assert!(rotated.y() < 0.0);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_with_zero_angle() {
        let axis = UnitVector3A::unit_x();
        let angle = 0.0;
        let quat = UnitQuaternionA::from_axis_angle(&axis, angle);

        // Zero angle should give identity rotation
        assert_abs_diff_eq!(quat.real(), 1.0, epsilon = EPSILON);
        let imag = quat.imag();
        assert_abs_diff_eq!(imag.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_with_full_rotation() {
        let axis = UnitVector3A::unit_y();
        let angle = 2.0 * PI;
        let quat = UnitQuaternionA::from_axis_angle(&axis, angle);

        let vector = Vector3A::new(1.0, 0.0, 1.0);
        let rotated = quat.rotate_vector(&vector);

        // Full rotation should return to original position
        assert_abs_diff_eq!(rotated.x(), vector.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), vector.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), vector.z(), epsilon = EPSILON);
    }

    #[test]
    fn quaternion_addition_with_itself() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat + &quat;

        assert_eq!(result.real(), 8.0);
        let imag = result.imag();
        assert_eq!(imag.x(), 2.0);
        assert_eq!(imag.y(), 4.0);
        assert_eq!(imag.z(), 6.0);
    }

    #[test]
    fn quaternion_subtraction_with_itself() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat - &quat;

        assert_abs_diff_eq!(result.real(), 0.0, epsilon = EPSILON);
        let imag = result.imag();
        assert_abs_diff_eq!(imag.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_double_negation_gives_original() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 6.0);
        let double_negated = -(-quat);

        assert_abs_diff_eq!(double_negated.real(), quat.real(), epsilon = EPSILON);
        let quat_imag = quat.imag();
        let double_neg_imag = double_negated.imag();
        assert_abs_diff_eq!(double_neg_imag.x(), quat_imag.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(double_neg_imag.y(), quat_imag.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(double_neg_imag.z(), quat_imag.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_parallel_axes() {
        let axis = UnitVector3A::unit_x();
        let rotation = UnitQuaternionA::rotation_between_axes(&axis, &axis);

        // Rotating from an axis to itself should be identity
        assert_abs_diff_eq!(rotation.real(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_opposite_axes() {
        let axis_x = UnitVector3A::unit_x();
        let axis_neg_x = UnitVector3A::neg_unit_x();

        let rotation = UnitQuaternionA::rotation_between_axes(&axis_x, &axis_neg_x);

        // Rotating from X to -X should be 180 degrees
        assert_abs_diff_eq!(rotation.angle().abs(), PI, epsilon = EPSILON);
    }

    #[test]
    fn non_aligned_quaternion_default_works() {
        let quat = Quaternion::default();
        assert_eq!(quat.real(), 1.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn non_aligned_unit_quaternion_default_works() {
        let quat = UnitQuaternion::default();
        assert_eq!(quat.real(), 1.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn non_aligned_quaternion_from_imag_works() {
        let imag = Vector3::new(1.0, 2.0, 3.0);
        let quat = Quaternion::from_imag(imag);

        assert_eq!(quat.real(), 0.0);
        assert_eq!(quat.imag(), &imag);
    }

    #[test]
    fn non_aligned_unit_quaternion_identity_works() {
        let identity = UnitQuaternion::identity();
        assert_eq!(identity.real(), 1.0);
        let imag = identity.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    // General trait tests
    #[test]
    fn quaternion_operations_with_different_reference_combinations_work() {
        let q1 = QuaternionA::from_parts(Vector3A::new(0.0, 0.0, 0.0), 1.0);
        let q2 = QuaternionA::from_parts(Vector3A::new(1.0, 0.0, 0.0), 0.0);

        // Test all combinations of reference/owned for binary operations
        let _result1 = &q1 + &q2; // ref + ref
        let _result2 = &q1 + q2; // ref + owned
        let _result3 = q1 + &q2; // owned + ref
        let _result4 = q1 + q2; // owned + owned

        // Recreate since they were moved
        let q1 = QuaternionA::from_parts(Vector3A::new(0.0, 0.0, 0.0), 1.0);
        let q2 = QuaternionA::from_parts(Vector3A::new(1.0, 0.0, 0.0), 0.0);

        let _result5 = &q1 * &q2; // ref * ref
        let _result6 = &q1 * q2; // ref * owned
        let _result7 = q1 * &q2; // owned * ref
        let _result8 = q1 * q2; // owned * owned
    }

    #[test]
    fn unit_quaternion_operations_with_different_reference_combinations_work() {
        let u1 = UnitQuaternionA::identity();
        let u2 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_x(), PI / 4.0);

        // Test all combinations for multiplication
        let _result1 = &u1 * &u2; // ref * ref
        let _result2 = &u1 * u2; // ref * owned
        let _result3 = u1 * &u2; // owned * ref
        let _result4 = u1 * u2; // owned * owned
    }

    #[test]
    fn quaternion_rotation_composition_is_associative() {
        let q1 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_x(), 0.1);
        let q2 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_y(), 0.2);
        let q3 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), 0.3);

        let left_assoc = &(&q1 * &q2) * &q3;
        let right_assoc = &q1 * &(&q2 * &q3);

        // Quaternion multiplication should be associative
        assert_abs_diff_eq!(left_assoc.real(), right_assoc.real(), epsilon = EPSILON);
        let left_imag = left_assoc.imag();
        let right_imag = right_assoc.imag();
        assert_abs_diff_eq!(left_imag.x(), right_imag.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(left_imag.y(), right_imag.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(left_imag.z(), right_imag.z(), epsilon = EPSILON);
    }

    #[test]
    fn quaternion_rotation_preserves_vector_length() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 3.0);
        let vector = Vector3A::new(2.0, 3.0, 4.0);
        let original_length = vector.norm();

        let rotated = quat.rotate_vector(&vector);
        let rotated_length = rotated.norm();

        assert_abs_diff_eq!(original_length, rotated_length, epsilon = EPSILON);
    }

    #[test]
    fn quaternion_identity_is_neutral_element() {
        let identity = UnitQuaternionA::identity();
        let test_quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_x(), PI / 6.0);

        let left_mult = &identity * &test_quat;
        let right_mult = &test_quat * &identity;

        // Identity should be neutral: I * q = q * I = q
        assert_abs_diff_eq!(left_mult.real(), test_quat.real(), epsilon = EPSILON);
        assert_abs_diff_eq!(right_mult.real(), test_quat.real(), epsilon = EPSILON);
    }

    #[test]
    fn quaternion_inverse_is_correct() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_y(), PI / 4.0);
        let inverse = quat.inverse();

        let vector = Vector3A::new(1.0, 0.0, 0.0);
        let rotated = quat.rotate_vector(&vector);
        let back_rotated = inverse.rotate_vector(&rotated);

        // q^-1 * q * v = v
        assert_abs_diff_eq!(back_rotated.x(), vector.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_rotated.y(), vector.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_rotated.z(), vector.z(), epsilon = EPSILON);
    }

    #[test]
    fn quaternion_axis_angle_identity_has_no_rotation() {
        let identity = UnitQuaternionA::identity();

        // Identity quaternion should have no axis (returns None) or angle of 0
        let angle = identity.angle();
        assert_abs_diff_eq!(angle, 0.0, epsilon = EPSILON);
    }

    #[test]
    fn quaternion_matrix_conversion_preserves_rotation() {
        let quat = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_x(), PI / 4.0);
        let matrix = quat.to_rotation_matrix();

        let vector = Vector3A::new(0.0, 1.0, 0.0);
        let quat_rotated = quat.rotate_vector(&vector);
        let matrix_rotated = &matrix * &vector;

        assert_abs_diff_eq!(quat_rotated.x(), matrix_rotated.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_rotated.y(), matrix_rotated.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_rotated.z(), matrix_rotated.z(), epsilon = EPSILON);
    }

    // Tests for non-aligned Quaternion type
    #[test]
    fn quaternion_from_vector_works() {
        let vector = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let quat = Quaternion::from_vector(vector);

        assert_eq!(quat.real(), 4.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 1.0);
        assert_eq!(imag.y(), 2.0);
        assert_eq!(imag.z(), 3.0);
    }

    #[test]
    fn quaternion_from_real_works() {
        let quat = Quaternion::from_real(5.0);

        assert_eq!(quat.real(), 5.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn quaternion_aligned_conversion_works() {
        let quat = Quaternion::from_parts(Vector3::new(1.0, 2.0, 3.0), 4.0);
        let aligned = quat.aligned();

        assert_eq!(aligned.real(), 4.0);
        let aligned_imag = aligned.imag();
        assert_eq!(aligned_imag.x(), 1.0);
        assert_eq!(aligned_imag.y(), 2.0);
        assert_eq!(aligned_imag.z(), 3.0);
    }

    // Tests for QuaternionA additional methods
    #[test]
    fn quaternion_a_from_vector_works() {
        let vector = Vector4A::new(2.0, 3.0, 4.0, 1.0);
        let quat = QuaternionA::from_vector(vector);

        assert_eq!(quat.real(), 1.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 2.0);
        assert_eq!(imag.y(), 3.0);
        assert_eq!(imag.z(), 4.0);
    }

    #[test]
    fn quaternion_a_from_real_works() {
        let quat = QuaternionA::from_real(7.0);

        assert_eq!(quat.real(), 7.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn quaternion_a_unaligned_conversion_works() {
        let quat_a = QuaternionA::from_parts(Vector3A::new(5.0, 6.0, 7.0), 8.0);
        let unaligned = quat_a.unaligned();

        assert_eq!(unaligned.real(), 8.0);
        let unaligned_imag = unaligned.imag();
        assert_eq!(unaligned_imag.x(), 5.0);
        assert_eq!(unaligned_imag.y(), 6.0);
        assert_eq!(unaligned_imag.z(), 7.0);
    }

    // Tests for QuaternionA arithmetic operations
    #[test]
    fn quaternion_a_subtraction_works() {
        let q1 = QuaternionA::from_parts(Vector3A::new(4.0, 5.0, 6.0), 3.0);
        let q2 = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 1.0);

        let result = &q1 - &q2;
        assert_eq!(result.real(), 2.0);
        let result_imag = result.imag();
        assert_eq!(result_imag.x(), 3.0);
        assert_eq!(result_imag.y(), 3.0);
        assert_eq!(result_imag.z(), 3.0);
    }

    #[test]
    fn quaternion_a_scalar_multiplication_works() {
        let quat = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let scalar = 2.0;

        let result1 = &quat * scalar;
        let result2 = scalar * &quat;

        assert_eq!(result1.real(), 8.0);
        assert_eq!(result2.real(), 8.0);

        let result1_imag = result1.imag();
        let result2_imag = result2.imag();
        assert_eq!(result1_imag.x(), 2.0);
        assert_eq!(result2_imag.x(), 2.0);
        assert_eq!(result1_imag.y(), 4.0);
        assert_eq!(result2_imag.y(), 4.0);
        assert_eq!(result1_imag.z(), 6.0);
        assert_eq!(result2_imag.z(), 6.0);
    }

    #[test]
    fn quaternion_a_scalar_division_works() {
        let quat = QuaternionA::from_parts(Vector3A::new(2.0, 4.0, 6.0), 8.0);
        let scalar = 2.0;

        let result = &quat / scalar;
        assert_eq!(result.real(), 4.0);
        let result_imag = result.imag();
        assert_eq!(result_imag.x(), 1.0);
        assert_eq!(result_imag.y(), 2.0);
        assert_eq!(result_imag.z(), 3.0);
    }

    #[test]
    fn quaternion_a_assignment_operations_work() {
        let mut q1 = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let q2 = QuaternionA::from_parts(Vector3A::new(1.0, 1.0, 1.0), 1.0);

        q1 += q2;
        assert_eq!(q1.real(), 5.0);
        let q1_imag = q1.imag();
        assert_eq!(q1_imag.x(), 2.0);
        assert_eq!(q1_imag.y(), 3.0);
        assert_eq!(q1_imag.z(), 4.0);

        q1 -= q2;
        assert_eq!(q1.real(), 4.0);
        let q1_imag = q1.imag();
        assert_eq!(q1_imag.x(), 1.0);
        assert_eq!(q1_imag.y(), 2.0);
        assert_eq!(q1_imag.z(), 3.0);

        q1 *= 2.0;
        assert_eq!(q1.real(), 8.0);
        let q1_imag = q1.imag();
        assert_eq!(q1_imag.x(), 2.0);
        assert_eq!(q1_imag.y(), 4.0);
        assert_eq!(q1_imag.z(), 6.0);

        q1 /= 2.0;
        assert_eq!(q1.real(), 4.0);
        let q1_imag = q1.imag();
        assert_eq!(q1_imag.x(), 1.0);
        assert_eq!(q1_imag.y(), 2.0);
        assert_eq!(q1_imag.z(), 3.0);
    }

    // Tests for non-aligned UnitQuaternion type
    #[test]
    fn unit_quaternion_as_quaternion_works() {
        let unit = UnitQuaternion::unchecked_from(Quaternion::from_parts(
            Vector3::new(0.0, 0.0, 0.0),
            1.0,
        ));
        let quat_ref = unit.as_quaternion();

        assert_eq!(quat_ref.real(), 1.0);
        let imag = quat_ref.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn unit_quaternion_aligned_conversion_works() {
        let unit = UnitQuaternion::identity();
        let aligned = unit.aligned();

        assert_eq!(aligned.real(), 1.0);
        let aligned_imag = aligned.imag();
        assert_eq!(aligned_imag.x(), 0.0);
        assert_eq!(aligned_imag.y(), 0.0);
        assert_eq!(aligned_imag.z(), 0.0);
    }

    #[test]
    fn unit_quaternion_a_unaligned_conversion_works() {
        let unit_a = UnitQuaternionA::identity();
        let unaligned = unit_a.unaligned();

        assert_eq!(unaligned.real(), 1.0);
        let unaligned_imag = unaligned.imag();
        assert_eq!(unaligned_imag.x(), 0.0);
        assert_eq!(unaligned_imag.y(), 0.0);
        assert_eq!(unaligned_imag.z(), 0.0);
    }

    // Tests for UnitQuaternionA scalar operations
    #[test]
    fn unit_quaternion_a_scalar_multiplication_gives_quaternion_a() {
        let unit = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 4.0);
        let scalar = 2.0;

        let result1 = &unit * scalar;
        let result2 = scalar * &unit;

        // Result should be QuaternionA (not unit anymore)
        assert_abs_diff_eq!(result1.real(), unit.real() * scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result2.real(), unit.real() * scalar, epsilon = EPSILON);

        let unit_imag = unit.imag();
        let result1_imag = result1.imag();
        let result2_imag = result2.imag();

        assert_abs_diff_eq!(result1_imag.x(), unit_imag.x() * scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result2_imag.x(), unit_imag.x() * scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result1_imag.y(), unit_imag.y() * scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result2_imag.y(), unit_imag.y() * scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result1_imag.z(), unit_imag.z() * scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result2_imag.z(), unit_imag.z() * scalar, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_a_scalar_division_gives_quaternion_a() {
        let unit = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_x(), PI / 3.0);
        let scalar = 0.5;

        let result = &unit / scalar;

        // Result should be QuaternionA (not unit anymore)
        assert_abs_diff_eq!(result.real(), unit.real() / scalar, epsilon = EPSILON);

        let unit_imag = unit.imag();
        let result_imag = result.imag();

        assert_abs_diff_eq!(result_imag.x(), unit_imag.x() / scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result_imag.y(), unit_imag.y() / scalar, epsilon = EPSILON);
        assert_abs_diff_eq!(result_imag.z(), unit_imag.z() / scalar, epsilon = EPSILON);
    }

    // Tests for From trait implementations
    #[test]
    fn conversion_from_quaternion_a_to_quaternion_works() {
        let quat_a = QuaternionA::from_parts(Vector3A::new(1.0, 2.0, 3.0), 4.0);
        let quat = quat_a.unaligned();

        assert_eq!(quat.real(), 4.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 1.0);
        assert_eq!(imag.y(), 2.0);
        assert_eq!(imag.z(), 3.0);
    }

    #[test]
    fn conversion_from_quaternion_to_quaternion_a_works() {
        let quat = Quaternion::from_parts(Vector3::new(5.0, 6.0, 7.0), 8.0);
        let quat_a = quat.aligned();

        assert_eq!(quat_a.real(), 8.0);
        let imag = quat_a.imag();
        assert_eq!(imag.x(), 5.0);
        assert_eq!(imag.y(), 6.0);
        assert_eq!(imag.z(), 7.0);
    }

    #[test]
    fn conversion_from_unit_quaternion_a_to_unit_quaternion_works() {
        let unit_a = UnitQuaternionA::identity();
        let unit = unit_a.unaligned();

        assert_eq!(unit.real(), 1.0);
        let imag = unit.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn conversion_from_unit_quaternion_to_unit_quaternion_a_works() {
        let unit = UnitQuaternion::identity();
        let unit_a = unit.aligned();

        assert_eq!(unit_a.real(), 1.0);
        let imag = unit_a.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn quaternion_a_quaternion_multiplication_assignment_works() {
        let mut q1 = QuaternionA::from_parts(Vector3A::new(1.0, 0.0, 0.0), 0.0);
        let q2 = QuaternionA::from_parts(Vector3A::new(0.0, 1.0, 0.0), 0.0);

        q1 *= q2;

        // i * j = k
        assert_abs_diff_eq!(q1.real(), 0.0, epsilon = EPSILON);
        let q1_imag = q1.imag();
        assert_abs_diff_eq!(q1_imag.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(q1_imag.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(q1_imag.z(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_a_multiplication_assignment_works() {
        let mut q1 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 4.0);
        let q2 = UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_z(), PI / 4.0);

        q1 *= q2;

        // Two 45-degree rotations should equal one 90-degree rotation
        assert_abs_diff_eq!(q1.angle(), PI / 2.0, epsilon = EPSILON);
    }
}
