//! Quaternions.

use crate::{
    matrix::{Matrix3, Matrix4},
    point::Point3,
    vector::{UnitVector3, Vector3, Vector3P, Vector4, Vector4P},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;
use std::{fmt, ops::Mul};

/// A quaternion.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned
/// [`QuaternionP`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
pub struct Quaternion {
    inner: glam::Quat,
}

/// A quaternion. This is the "packed" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`Quaternion`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 4]", from = "[f32; 4]")
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct QuaternionP {
    imag: Vector3P,
    real: f32,
}

/// A quaternion of unit length, representing a rotation.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned
/// [`UnitQuaternionP`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
pub struct UnitQuaternion {
    inner: glam::Quat,
}

/// A quaternion of unit length, representing a rotation. This is the "packed"
/// version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`UnitQuaternion`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 4]", from = "[f32; 4]")
)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct UnitQuaternionP {
    imag: Vector3P,
    real: f32,
}

impl Quaternion {
    /// Creates a quaternion with the given imaginary and real parts.
    #[inline]
    pub fn from_parts(imag: Vector3, real: f32) -> Self {
        Self::wrap(glam::Quat::from_xyzw(imag.x(), imag.y(), imag.z(), real))
    }

    /// Creates a quaternion from the given vector, with the last component
    /// representing the real part.
    #[inline]
    pub const fn from_vector(vector: Vector4) -> Self {
        Self::wrap(glam::Quat::from_vec4(vector.unwrap()))
    }

    /// Creates a quaternion with the given imaginary part and zero real part.
    #[inline]
    pub fn from_imag(imag: Vector3) -> Self {
        Self::wrap(glam::Quat::from_xyzw(imag.x(), imag.y(), imag.z(), 0.0))
    }

    /// Creates a quaternion with the given real part and zero imaginary part.
    #[inline]
    pub const fn from_real(real: f32) -> Self {
        Self::wrap(glam::Quat::from_xyzw(0.0, 0.0, 0.0, real))
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub fn imag(&self) -> Vector3 {
        Vector3::wrap(self.inner.xyz().to_vec3a())
    }

    /// The real part of the quaternion.
    #[inline]
    pub fn real(&self) -> f32 {
        self.inner.w
    }

    /// Converts the quaternion to the 4-byte aligned cache-friendly
    /// [`QuaternionP`].
    #[inline]
    pub fn pack(&self) -> QuaternionP {
        QuaternionP::from_parts(self.imag().pack(), self.real())
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

impl Default for Quaternion {
    fn default() -> Self {
        Self::from_real(1.0)
    }
}

impl_binop!(Add, add, Quaternion, Quaternion, Quaternion, |a, b| {
    Quaternion::wrap(a.inner.add(b.inner))
});

impl_binop!(Sub, sub, Quaternion, Quaternion, Quaternion, |a, b| {
    Quaternion::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Quaternion, Quaternion, Quaternion, |a, b| {
    Quaternion::wrap(a.inner.mul_quat(b.inner))
});

impl_binop!(Mul, mul, Quaternion, f32, Quaternion, |a, b| {
    Quaternion::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Quaternion, Quaternion, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Quaternion, f32, Quaternion, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, Quaternion, Quaternion, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Quaternion, Quaternion, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Quaternion, Quaternion, |a, b| {
    a.inner.mul_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Quaternion, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Quaternion, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Quaternion, Quaternion, |val| {
    Quaternion::wrap(val.inner.neg())
});

impl_abs_diff_eq!(Quaternion, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Quaternion, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Quaternion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Quaternion")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .field("w", &self.inner.w)
            .finish()
    }
}

impl QuaternionP {
    /// Creates a quaternion with the given imaginary and real parts.
    #[inline]
    pub const fn from_parts(imag: Vector3P, real: f32) -> Self {
        Self { imag, real }
    }

    /// Creates a quaternion from the given vector, with the last component
    /// representing the real part.
    #[inline]
    pub const fn from_vector(vector: Vector4P) -> Self {
        Self {
            imag: vector.xyz(),
            real: vector.w(),
        }
    }

    /// Creates a quaternion with the given imaginary part and zero real part.
    #[inline]
    pub const fn from_imag(imag: Vector3P) -> Self {
        Self::from_parts(imag, 0.0)
    }

    /// Creates a quaternion with the given real part and zero imaginary part.
    #[inline]
    pub const fn from_real(real: f32) -> Self {
        Self::from_parts(Vector3P::zeros(), real)
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub fn imag(&self) -> &Vector3P {
        &self.imag
    }

    /// The real part of the quaternion.
    #[inline]
    pub fn real(&self) -> f32 {
        self.real
    }

    /// Converts the quaternion to the 16-byte aligned SIMD-friendly
    /// [`Quaternion`].
    #[inline]
    pub fn unpack(&self) -> Quaternion {
        Quaternion::from_parts(self.imag().unpack(), self.real())
    }
}

impl Default for QuaternionP {
    fn default() -> Self {
        Self::from_real(1.0)
    }
}

impl From<QuaternionP> for [f32; 4] {
    fn from(q: QuaternionP) -> [f32; 4] {
        [q.imag.x(), q.imag.y(), q.imag.z(), q.real]
    }
}

impl From<[f32; 4]> for QuaternionP {
    fn from(arr: [f32; 4]) -> QuaternionP {
        QuaternionP::from_parts(Vector3P::new(arr[0], arr[1], arr[2]), arr[3])
    }
}

impl_abs_diff_eq!(QuaternionP, |a, b, epsilon| {
    a.imag.abs_diff_eq(&b.imag, epsilon) && a.real.abs_diff_eq(&b.real, epsilon)
});

impl_relative_eq!(QuaternionP, |a, b, epsilon, max_relative| {
    a.imag.relative_eq(&b.imag, epsilon, max_relative)
        && a.real.relative_eq(&b.real, epsilon, max_relative)
});

impl UnitQuaternion {
    /// Creates a unit quaternion representing the identity rotation.
    #[inline]
    pub const fn identity() -> Self {
        Self::wrap(glam::Quat::IDENTITY)
    }

    /// Converts the given quaternion to a unit quaternion, assuming it is
    /// already normalized.
    #[inline]
    pub const fn unchecked_from(quaternion: Quaternion) -> Self {
        Self::wrap(quaternion.unwrap())
    }

    /// Creates a unit quaternion by normalizing the given quaternion. If the
    /// quaternion has zero length, the result will be non-finite.
    #[inline]
    pub fn normalized_from(quaternion: Quaternion) -> Self {
        Self::wrap(quaternion.unwrap().normalize())
    }

    /// Creates a unit quaternion representing a rotation of the given angle (in
    /// radians) about the given axis.
    #[inline]
    pub fn from_axis_angle(axis: &UnitVector3, angle: f32) -> Self {
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
    pub fn rotation_between_axes(from: &UnitVector3, to: &UnitVector3) -> Self {
        Self::wrap(glam::Quat::from_rotation_arc(
            from.unwrap().to_vec3(),
            to.unwrap().to_vec3(),
        ))
    }

    /// Creates a unit quaternion reprenting the rotation aligning the positive
    /// z-axis with the given view direction and the positive y-axis with the
    /// given up direction.
    #[inline]
    pub fn look_to_rh(dir: &UnitVector3, up: &UnitVector3) -> Self {
        Self::wrap(glam::Quat::look_to_rh(
            dir.unwrap().to_vec3(),
            up.unwrap().to_vec3(),
        ))
    }

    /// Creates a unit quaternion representing the orientation of the reference
    /// frame with the given three basis vectors. The vectors are assumed
    /// normalized and perpendicular.
    #[inline]
    pub fn from_basis_unchecked(basis: &[Vector3; 3]) -> Self {
        // `glam::Mat3A` is column-major, so we can just cast the reference
        Self::wrap(glam::Quat::from_mat3a(bytemuck::cast_ref(basis)))
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub fn imag(&self) -> Vector3 {
        Vector3::wrap(self.inner.xyz().to_vec3a())
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
    pub fn axis_angle(&self) -> (UnitVector3, f32) {
        let (axis, angle) = self.inner.to_axis_angle();
        (UnitVector3::wrap(axis.to_vec3a()), angle)
    }

    /// Computes the axis of this rotation.
    #[inline]
    pub fn axis(&self) -> UnitVector3 {
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
    pub fn to_rotation_matrix(&self) -> Matrix3 {
        Matrix3::wrap(glam::Mat3A::from_quat(self.inner))
    }

    /// Converts the quaternion to a 4x4 homogeneous matrix.
    #[inline]
    pub fn to_homogeneous_matrix(&self) -> Matrix4 {
        Matrix4::wrap(glam::Mat4::from_quat(self.inner))
    }

    /// Applies the rotation to the given point.
    #[inline]
    pub fn rotate_point(&self, point: &Point3) -> Point3 {
        Point3::wrap(self.inner.mul_vec3a(point.unwrap()))
    }

    /// Applies the rotation to the given vector.
    #[inline]
    pub fn rotate_vector(&self, vector: &Vector3) -> Vector3 {
        Vector3::wrap(self.inner.mul_vec3a(vector.unwrap()))
    }

    /// Applies the rotation to the given unit vector.
    #[inline]
    pub fn rotate_unit_vector(&self, vector: &UnitVector3) -> UnitVector3 {
        UnitVector3::wrap(self.inner.mul_vec3a(vector.unwrap()))
    }

    /// This unit quaternion as a [`Quaternion`].
    #[inline]
    pub fn as_quaternion(&self) -> &Quaternion {
        bytemuck::cast_ref(self)
    }

    /// Converts the quaternion to the 4-byte aligned cache-friendly
    /// [`UnitQuaternionP`].
    #[inline]
    pub fn pack(&self) -> UnitQuaternionP {
        UnitQuaternionP::unchecked_from(QuaternionP::from_parts(
            Vector3P::from_glam(self.inner.xyz()),
            self.real(),
        ))
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Quat) -> Self {
        Self { inner }
    }
}

impl Default for UnitQuaternion {
    fn default() -> Self {
        Self::identity()
    }
}

impl_binop!(
    Mul,
    mul,
    UnitQuaternion,
    UnitQuaternion,
    UnitQuaternion,
    |a, b| { UnitQuaternion::wrap(a.inner.mul_quat(b.inner)) }
);

impl_binop!(Mul, mul, UnitQuaternion, f32, Quaternion, |a, b| {
    Quaternion::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, UnitQuaternion, Quaternion, |a, b| {
    b.mul(*a)
});

impl_binop!(Div, div, UnitQuaternion, f32, Quaternion, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(
    MulAssign,
    mul_assign,
    UnitQuaternion,
    UnitQuaternion,
    |a, b| {
        a.inner.mul_assign(b.inner);
    }
);

impl_unary_op!(Neg, neg, UnitQuaternion, UnitQuaternion, |val| {
    UnitQuaternion::wrap(val.inner.neg())
});

impl_abs_diff_eq!(UnitQuaternion, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(UnitQuaternion, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for UnitQuaternion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitQuaternion")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .field("w", &self.inner.w)
            .finish()
    }
}

impl UnitQuaternionP {
    /// Creates a unit quaternion representing the identity rotation.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            imag: Vector3P::zeros(),
            real: 1.0,
        }
    }

    /// Converts the given quaternion to a unit quaternion, assuming it is
    /// already normalized.
    #[inline]
    pub const fn unchecked_from(quaternion: QuaternionP) -> Self {
        Self {
            imag: quaternion.imag,
            real: quaternion.real,
        }
    }

    /// The imaginary part of the quaternion.
    #[inline]
    pub const fn imag(&self) -> &Vector3P {
        &self.imag
    }

    /// The real part of the quaternion.
    #[inline]
    pub const fn real(&self) -> f32 {
        self.real
    }

    /// This unit quaternion as a [`QuaternionP`].
    #[inline]
    pub fn as_quaternion(&self) -> &QuaternionP {
        bytemuck::cast_ref(self)
    }

    /// Converts the quaternion to the 16-byte aligned SIMD-friendly
    /// [`UnitQuaternion`].
    #[inline]
    pub fn unpack(&self) -> UnitQuaternion {
        UnitQuaternion::unchecked_from(Quaternion::from_parts(self.imag().unpack(), self.real()))
    }
}

impl Default for UnitQuaternionP {
    fn default() -> Self {
        Self::identity()
    }
}

impl From<UnitQuaternionP> for [f32; 4] {
    fn from(q: UnitQuaternionP) -> [f32; 4] {
        [q.imag.x(), q.imag.y(), q.imag.z(), q.real]
    }
}

impl From<[f32; 4]> for UnitQuaternionP {
    fn from(arr: [f32; 4]) -> UnitQuaternionP {
        UnitQuaternionP::unchecked_from(QuaternionP::from_parts(
            Vector3P::new(arr[0], arr[1], arr[2]),
            arr[3],
        ))
    }
}

impl_abs_diff_eq!(UnitQuaternionP, |a, b, epsilon| {
    a.imag.abs_diff_eq(&b.imag, epsilon) && a.real.abs_diff_eq(&b.real, epsilon)
});

impl_relative_eq!(UnitQuaternionP, |a, b, epsilon, max_relative| {
    a.imag.relative_eq(&b.imag, epsilon, max_relative)
        && a.real.relative_eq(&b.real, epsilon, max_relative)
});

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
impl_roc_for_library_provided_primitives! {
//  Type               Pkg   Parents  Module          Roc name        Postfix  Precision
    UnitQuaternionP => core, None,    UnitQuaternion, UnitQuaternion, None,    PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use crate::consts::f32::PI;
    use approx::assert_abs_diff_eq;

    const EPSILON: f32 = 1e-6;

    // === Quaternion Tests (SIMD-aligned) ===

    #[test]
    fn quaternion_addition_works() {
        let q1 = Quaternion::from_parts(Vector3::new(2.0, 3.0, 4.0), 1.0);
        let q2 = Quaternion::from_parts(Vector3::new(1.0, 1.0, 1.0), 2.0);

        let result = &q1 + &q2;
        assert_eq!(
            result,
            Quaternion::from_parts(Vector3::new(3.0, 4.0, 5.0), 3.0)
        );
    }

    #[test]
    fn quaternion_multiplication_works() {
        let q1 = Quaternion::from_parts(Vector3::new(0.0, 0.0, 0.0), 1.0);
        let q2 = Quaternion::from_parts(Vector3::new(1.0, 0.0, 0.0), 0.0);

        let result = &q1 * &q2;
        assert_abs_diff_eq!(result.real(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result.imag().x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result.imag().y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result.imag().z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn quaternion_addition_with_itself_doubles_components() {
        let quat = Quaternion::from_parts(Vector3::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat + &quat;
        assert_eq!(
            result,
            Quaternion::from_parts(Vector3::new(2.0, 4.0, 6.0), 8.0)
        );
    }

    #[test]
    fn quaternion_subtraction_with_itself_gives_zero() {
        let quat = Quaternion::from_parts(Vector3::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat - &quat;
        assert_abs_diff_eq!(result.real(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result.imag().x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result.imag().y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(result.imag().z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn quaternion_scalar_multiplication_by_zero_gives_zero() {
        let quat = Quaternion::from_parts(Vector3::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat * 0.0;
        assert_eq!(
            result,
            Quaternion::from_parts(Vector3::new(0.0, 0.0, 0.0), 0.0)
        );
    }

    #[test]
    fn quaternion_scalar_multiplication_by_one_gives_same_quaternion() {
        let quat = Quaternion::from_parts(Vector3::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat * 1.0;
        assert_eq!(result, quat);
    }

    #[test]
    fn quaternion_scalar_multiplication_by_negative_negates_components() {
        let quat = Quaternion::from_parts(Vector3::new(1.0, 2.0, 3.0), 4.0);
        let result = &quat * -1.0;
        assert_eq!(
            result,
            Quaternion::from_parts(Vector3::new(-1.0, -2.0, -3.0), -4.0)
        );
    }

    #[test]
    fn converting_quaternion_to_aligned_and_back_preserves_data() {
        let quat = Quaternion::from_parts(Vector3::new(1.0, 2.0, 3.0), 4.0);
        let packed = quat.pack();
        assert_eq!(packed.unpack(), quat);
    }

    // === UnitQuaternion Tests (SIMD-aligned) ===

    #[test]
    fn unit_quaternion_from_axis_angle_works() {
        let axis = UnitVector3::unit_z();
        let angle = PI / 2.0;
        let unit = UnitQuaternion::from_axis_angle(&axis, angle);

        let (extracted_axis, extracted_angle) = unit.axis_angle();
        assert_abs_diff_eq!(extracted_angle, angle, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_axis.z(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_with_negative_angle_rotates_opposite_direction() {
        let axis = UnitVector3::unit_z();
        let angle = -PI / 4.0;
        let quat = UnitQuaternion::from_axis_angle(&axis, angle);

        let vector = Vector3::new(1.0, 0.0, 0.0);
        let rotated = quat.rotate_vector(&vector);

        assert!(rotated.x() > 0.0);
        assert!(rotated.y() < 0.0);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_with_zero_angle_gives_identity() {
        let axis = UnitVector3::unit_x();
        let angle = 0.0;
        let quat = UnitQuaternion::from_axis_angle(&axis, angle);

        assert_abs_diff_eq!(quat.real(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(quat.imag().x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(quat.imag().y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(quat.imag().z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_with_full_rotation_preserves_vector() {
        let axis = UnitVector3::unit_y();
        let angle = 2.0 * PI;
        let quat = UnitQuaternion::from_axis_angle(&axis, angle);

        let vector = Vector3::new(1.0, 0.0, 1.0);
        let rotated = quat.rotate_vector(&vector);

        assert_abs_diff_eq!(rotated.x(), vector.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), vector.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), vector.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_euler_angles_roundtrip_preserves_angles() {
        let roll = 0.1;
        let pitch = 0.2;
        let yaw = 0.3;

        let quat = UnitQuaternion::from_euler_angles(roll, pitch, yaw);
        let (extracted_roll, extracted_pitch, extracted_yaw) = quat.euler_angles();

        assert_abs_diff_eq!(extracted_roll, roll, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_pitch, pitch, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_yaw, yaw, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_axes_works() {
        let axis_x = UnitVector3::unit_x();
        let axis_y = UnitVector3::unit_y();

        let rotation = UnitQuaternion::rotation_between_axes(&axis_x, &axis_y);

        let rotated = rotation.rotate_unit_vector(&axis_x);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_parallel_axes_gives_identity() {
        let axis = UnitVector3::unit_x();
        let rotation = UnitQuaternion::rotation_between_axes(&axis, &axis);

        assert_abs_diff_eq!(rotation.real(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_opposite_axes_gives_180_degrees() {
        let axis_x = UnitVector3::unit_x();
        let axis_neg_x = UnitVector3::neg_unit_x();

        let rotation = UnitQuaternion::rotation_between_axes(&axis_x, &axis_neg_x);

        assert_abs_diff_eq!(rotation.angle().abs(), PI, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_look_to_rh_with_standard_orientation_is_near_identity() {
        let dir = UnitVector3::neg_unit_z();
        let up = UnitVector3::unit_y();

        let look_at = UnitQuaternion::look_to_rh(&dir, &up);

        assert_abs_diff_eq!(look_at.real().abs(), 1.0, epsilon = 0.1);
    }

    #[test]
    fn unit_quaternion_from_basis_unchecked_with_standard_basis_gives_identity() {
        let basis = [
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];

        let quat = UnitQuaternion::from_basis_unchecked(&basis);

        assert_abs_diff_eq!(quat.real(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_inverse_multiplied_with_original_gives_identity() {
        let axis = UnitVector3::unit_z();
        let angle = PI / 4.0;
        let quat = UnitQuaternion::from_axis_angle(&axis, angle);
        let inverse = quat.inverse();

        let product = &quat * &inverse;
        assert_abs_diff_eq!(product.real(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(product.imag().x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(product.imag().y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(product.imag().z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_inverse_reverses_rotation() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), PI / 4.0);
        let inverse = quat.inverse();

        let vector = Vector3::new(1.0, 0.0, 0.0);
        let rotated = quat.rotate_vector(&vector);
        let back_rotated = inverse.rotate_vector(&rotated);

        assert_abs_diff_eq!(back_rotated.x(), vector.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_rotated.y(), vector.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_rotated.z(), vector.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_negation_represents_same_rotation() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0);
        let negated = -quat;

        let vector = Vector3::new(0.0, 1.0, 0.0);
        let rotated1 = quat.rotate_vector(&vector);
        let rotated2 = negated.rotate_vector(&vector);

        assert_abs_diff_eq!(rotated1.x(), rotated2.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated1.y(), rotated2.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated1.z(), rotated2.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_double_negation_gives_original() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 6.0);
        let double_negated = -(-quat);

        assert_abs_diff_eq!(double_negated.real(), quat.real(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            double_negated.imag().x(),
            quat.imag().x(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            double_negated.imag().y(),
            quat.imag().y(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            double_negated.imag().z(),
            quat.imag().z(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn unit_quaternion_axis_angle_extraction_preserves_inputs() {
        let original_axis = UnitVector3::unit_y();
        let original_angle = PI / 3.0;
        let quat = UnitQuaternion::from_axis_angle(&original_axis, original_angle);

        let (extracted_axis, extracted_angle) = quat.axis_angle();

        assert_abs_diff_eq!(extracted_angle, original_angle, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_axis.y(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_identity_has_zero_rotation_angle() {
        let identity = UnitQuaternion::identity();
        let angle = identity.angle();
        assert_abs_diff_eq!(angle, 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_rotation_matrix_preserves_rotation() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0);
        let matrix = quat.to_rotation_matrix();

        let vector = Vector3::new(0.0, 1.0, 0.0);
        let quat_rotated = quat.rotate_vector(&vector);
        let matrix_rotated = &matrix * &vector;

        assert_abs_diff_eq!(quat_rotated.x(), matrix_rotated.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_rotated.y(), matrix_rotated.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_rotated.z(), matrix_rotated.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_rotation_matrix_rotates_correctly() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let matrix = quat.to_rotation_matrix();

        let x_axis = Vector3::new(1.0, 0.0, 0.0);
        let rotated = &matrix * &x_axis;

        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_homogeneous_matrix_has_no_translation() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let matrix = quat.to_homogeneous_matrix();

        assert_abs_diff_eq!(matrix.element(0, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(1, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(2, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(3, 3), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotate_point_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let point = Point3::new(1.0, 0.0, 0.0);

        let rotated = quat.rotate_point(&point);

        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotate_vector_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let vector = Vector3::new(1.0, 0.0, 0.0);

        let rotated = quat.rotate_vector(&vector);

        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotate_unit_vector_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let unit_vector = UnitVector3::unit_x();

        let rotated = quat.rotate_unit_vector(&unit_vector);

        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_preserves_vector_length() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 3.0);
        let vector = Vector3::new(2.0, 3.0, 4.0);
        let original_length = vector.norm();

        let rotated = quat.rotate_vector(&vector);
        let rotated_length = rotated.norm();

        assert_abs_diff_eq!(original_length, rotated_length, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_multiplication_composes_rotations() {
        let q1 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 4.0);
        let q2 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 4.0);

        let result = &q1 * &q2;

        let expected = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);

        assert_abs_diff_eq!(result.angle(), expected.angle(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_multiplication_is_associative() {
        let q1 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), 0.1);
        let q2 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), 0.2);
        let q3 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 0.3);

        let left_assoc = &(&q1 * &q2) * &q3;
        let right_assoc = &q1 * &(&q2 * &q3);

        assert_abs_diff_eq!(left_assoc.real(), right_assoc.real(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            left_assoc.imag().x(),
            right_assoc.imag().x(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            left_assoc.imag().y(),
            right_assoc.imag().y(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            left_assoc.imag().z(),
            right_assoc.imag().z(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn unit_quaternion_identity_is_neutral_element() {
        let identity = UnitQuaternion::identity();
        let test_quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 6.0);

        let left_mult = &identity * &test_quat;
        let right_mult = &test_quat * &identity;

        assert_abs_diff_eq!(left_mult.real(), test_quat.real(), epsilon = EPSILON);
        assert_abs_diff_eq!(right_mult.real(), test_quat.real(), epsilon = EPSILON);
    }

    #[test]
    fn converting_unit_quaternion_to_aligned_and_back_preserves_data() {
        let unit = UnitQuaternion::identity();
        let packed = unit.pack();
        assert_eq!(packed.unpack().real(), unit.real());
    }

    // === QuaternionP Tests (packed) ===

    #[test]
    fn converting_quaternionp_to_aligned_and_back_preserves_data() {
        let quat = QuaternionP::from_parts(Vector3P::new(1.0, 2.0, 3.0), 4.0);
        let aligned = quat.unpack();
        assert_eq!(aligned.pack(), quat);
    }

    // === UnitQuaternionP Tests (packed) ===

    #[test]
    fn converting_unit_quaternionp_to_aligned_and_back_preserves_data() {
        let unit = UnitQuaternionP::identity();
        let aligned = unit.unpack();
        assert_eq!(aligned.real(), unit.real());
    }
}
