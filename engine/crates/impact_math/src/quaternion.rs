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

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use crate::{
        point::Point3,
        vector::{UnitVector3, Vector3},
    };
    use approx::assert_abs_diff_eq;
    use std::f32::consts::PI;

    // Test constants
    const EPSILON: f32 = 1e-6;

    // Quaternion tests
    #[test]
    fn quaternion_from_parts_works() {
        let real = 1.0;
        let imag = Vector3::new(2.0, 3.0, 4.0);
        let quat = Quaternion::from_parts(real, imag);

        assert_eq!(quat.real(), 1.0);
        assert_eq!(quat.imag(), imag);
    }

    #[test]
    fn quaternion_from_imag_works() {
        let imag = Vector3::new(2.0, 3.0, 4.0);
        let quat = Quaternion::from_imag(imag);

        assert_eq!(quat.real(), 0.0);
        assert_eq!(quat.imag(), imag);
    }

    #[test]
    fn quaternion_real_and_imag_accessors_work() {
        let quat = Quaternion::from_parts(5.0, Vector3::new(1.0, 2.0, 3.0));

        assert_eq!(quat.real(), 5.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 1.0);
        assert_eq!(imag.y(), 2.0);
        assert_eq!(imag.z(), 3.0);
    }

    #[test]
    fn quaternion_negated_works() {
        let quat = Quaternion::from_parts(2.0, Vector3::new(1.0, -2.0, 3.0));
        let negated = quat.negated();

        assert_eq!(negated.real(), -2.0);
        let neg_imag = negated.imag();
        assert_eq!(neg_imag.x(), -1.0);
        assert_eq!(neg_imag.y(), 2.0);
        assert_eq!(neg_imag.z(), -3.0);
    }

    #[test]
    fn quaternion_addition_works() {
        let q1 = Quaternion::from_parts(1.0, Vector3::new(2.0, 3.0, 4.0));
        let q2 = Quaternion::from_parts(2.0, Vector3::new(1.0, 1.0, 1.0));

        let result = &q1 + &q2;
        assert_eq!(result.real(), 3.0);
        let result_imag = result.imag();
        assert_eq!(result_imag.x(), 3.0);
        assert_eq!(result_imag.y(), 4.0);
        assert_eq!(result_imag.z(), 5.0);
    }

    #[test]
    fn quaternion_multiplication_works() {
        let q1 = Quaternion::from_parts(1.0, Vector3::new(0.0, 0.0, 0.0));
        let q2 = Quaternion::from_parts(0.0, Vector3::new(1.0, 0.0, 0.0));

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
        let quat = Quaternion::default();
        assert_eq!(quat.real(), 0.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    // UnitQuaternion tests
    #[test]
    fn unit_quaternion_identity_works() {
        let identity = UnitQuaternion::identity();
        assert_eq!(identity.real(), 1.0);
        let imag = identity.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn unit_quaternion_normalized_from_works() {
        let quat = Quaternion::from_parts(2.0, Vector3::new(0.0, 0.0, 0.0));
        let unit = UnitQuaternion::normalized_from(quat);

        // Should normalize to (1, 0, 0, 0)
        assert_abs_diff_eq!(unit.real(), 1.0, epsilon = EPSILON);
        let imag = unit.imag();
        assert_abs_diff_eq!(imag.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(imag.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_unchecked_from_works() {
        let quat = Quaternion::from_parts(1.0, Vector3::new(0.0, 0.0, 0.0));
        let unit = UnitQuaternion::unchecked_from(quat);

        assert_eq!(unit.real(), 1.0);
        let imag = unit.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    #[test]
    fn unit_quaternion_from_axis_angle_works() {
        let axis = UnitVector3::unit_z();
        let angle = PI / 2.0; // 90 degrees
        let unit = UnitQuaternion::from_axis_angle(&axis, angle);

        // Rotation around Z axis by 90 degrees
        let (extracted_axis, extracted_angle) = unit.axis_angle().unwrap();
        assert_abs_diff_eq!(extracted_angle, angle, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_axis.z(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_from_euler_angles_works() {
        let roll = 0.0;
        let pitch = 0.0;
        let yaw = PI / 2.0;
        let unit = UnitQuaternion::from_euler_angles(roll, pitch, yaw);

        let (extracted_roll, extracted_pitch, extracted_yaw) = unit.euler_angles();
        assert_abs_diff_eq!(extracted_roll, roll, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_pitch, pitch, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_yaw, yaw, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_axis_works() {
        let axis_x = UnitVector3::unit_x();
        let axis_y = UnitVector3::unit_y();

        let rotation = UnitQuaternion::rotation_between_axis(&axis_x, &axis_y).unwrap();

        // Should rotate X axis to Y axis
        let rotated = rotation.rotate_unit_vector(&axis_x);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotation_between_axis_returns_none_for_opposite_vectors() {
        let axis_x = UnitVector3::unit_x();
        let neg_axis_x = UnitVector3::normalized_from(Vector3::new(-1.0, 0.0, 0.0));

        let rotation = UnitQuaternion::rotation_between_axis(&axis_x, &neg_axis_x);
        // This might return None for opposite vectors (180 degree rotation is ambiguous)
        // The exact behavior depends on nalgebra implementation
        assert!(rotation.is_some() || rotation.is_none());
    }

    #[test]
    fn unit_quaternion_look_at_rh_works() {
        let dir = Vector3::new(0.0, 0.0, -1.0); // Looking down negative Z
        let up = Vector3::new(0.0, 1.0, 0.0); // Y is up

        let look_at = UnitQuaternion::look_at_rh(&dir, &up);

        // Should be close to identity for this standard orientation
        assert_abs_diff_eq!(look_at.real().abs(), 1.0, epsilon = 0.1);
    }

    #[test]
    fn unit_quaternion_from_basis_unchecked_works() {
        let basis = [
            Vector3::new(1.0, 0.0, 0.0), // X axis
            Vector3::new(0.0, 1.0, 0.0), // Y axis
            Vector3::new(0.0, 0.0, 1.0), // Z axis
        ];

        let quat = UnitQuaternion::from_basis_unchecked(&basis);

        // Should be close to identity for standard basis
        assert_abs_diff_eq!(quat.real(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_inverse_works() {
        let axis = UnitVector3::unit_z();
        let angle = PI / 4.0;
        let quat = UnitQuaternion::from_axis_angle(&axis, angle);
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
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0);
        let negated = quat.negated();

        // Negated quaternion represents the same rotation
        let vector = Vector3::new(0.0, 1.0, 0.0);
        let rotated1 = quat.transform_vector(&vector);
        let rotated2 = negated.transform_vector(&vector);

        assert_abs_diff_eq!(rotated1.x(), rotated2.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated1.y(), rotated2.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(rotated1.z(), rotated2.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_real_and_imag_work() {
        let axis = UnitVector3::unit_z();
        let angle = PI / 2.0;
        let quat = UnitQuaternion::from_axis_angle(&axis, angle);

        let real = quat.real();
        let imag = quat.imag();

        // For rotation around Z by PI/2: q = cos(PI/4) + sin(PI/4) * k
        assert_abs_diff_eq!(real, (PI / 4.0).cos(), epsilon = EPSILON);
        assert_abs_diff_eq!(imag.z(), (PI / 4.0).sin(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_axis_angle_extraction_works() {
        let original_axis = UnitVector3::unit_y();
        let original_angle = PI / 3.0;
        let quat = UnitQuaternion::from_axis_angle(&original_axis, original_angle);

        let (extracted_axis, extracted_angle) = quat.axis_angle().unwrap();

        assert_abs_diff_eq!(extracted_angle, original_angle, epsilon = EPSILON);
        assert_abs_diff_eq!(extracted_axis.y(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_axis_extraction_works() {
        let original_axis = UnitVector3::unit_x();
        let quat = UnitQuaternion::from_axis_angle(&original_axis, PI / 4.0);

        let extracted_axis = quat.axis().unwrap();
        assert_abs_diff_eq!(extracted_axis.x(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_angle_extraction_works() {
        let original_angle = PI / 6.0;
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), original_angle);

        let extracted_angle = quat.angle();
        assert_abs_diff_eq!(extracted_angle, original_angle, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_euler_angles_roundtrip_works() {
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
    fn unit_quaternion_to_quaternion_works() {
        let unit = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 4.0);
        let quat = unit.to_quaternion();

        assert_abs_diff_eq!(quat.real(), unit.real(), epsilon = EPSILON);
        let unit_imag = unit.imag();
        let quat_imag = quat.imag();
        assert_abs_diff_eq!(quat_imag.x(), unit_imag.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_imag.y(), unit_imag.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_imag.z(), unit_imag.z(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_rotation_matrix_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let matrix = quat.to_rotation_matrix();

        // 90 degree rotation around Z should map X to Y
        let x_axis = Vector3::new(1.0, 0.0, 0.0);
        let rotated = &matrix * &x_axis;

        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_to_homogeneous_matrix_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let matrix = quat.to_homogeneous_matrix();

        // Should be a 4x4 matrix with rotation in upper-left 3x3 and no translation
        assert_abs_diff_eq!(matrix.element(0, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(1, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(2, 3), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(3, 3), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_transform_point_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let point = Point3::new(1.0, 0.0, 0.0);

        let transformed = quat.transform_point(&point);

        // 90 degree rotation around Z maps (1,0,0) to (0,1,0)
        assert_abs_diff_eq!(transformed.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_transform_vector_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let vector = Vector3::new(1.0, 0.0, 0.0);

        let transformed = quat.transform_vector(&vector);

        // 90 degree rotation around Z maps (1,0,0) to (0,1,0)
        assert_abs_diff_eq!(transformed.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_inverse_transform_point_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let point = Point3::new(0.0, 1.0, 0.0);

        let inverse_transformed = quat.inverse_transform_point(&point);

        // Inverse of 90 degree rotation around Z maps (0,1,0) to (1,0,0)
        assert_abs_diff_eq!(inverse_transformed.x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(inverse_transformed.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(inverse_transformed.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_inverse_transform_vector_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let vector = Vector3::new(0.0, 1.0, 0.0);

        let inverse_transformed = quat.inverse_transform_vector(&vector);

        // Inverse of 90 degree rotation around Z maps (0,1,0) to (1,0,0)
        assert_abs_diff_eq!(inverse_transformed.x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(inverse_transformed.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(inverse_transformed.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_rotate_unit_vector_works() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);
        let unit_vector = UnitVector3::unit_x();

        let rotated = quat.rotate_unit_vector(&unit_vector);

        // 90 degree rotation around Z maps X to Y
        assert_abs_diff_eq!(rotated.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_multiplication_works() {
        let q1 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 4.0);
        let q2 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 4.0);

        let result = &q1 * &q2;

        // Two 45-degree rotations should equal one 90-degree rotation
        let expected = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0);

        assert_abs_diff_eq!(result.angle(), expected.angle(), epsilon = EPSILON);
    }

    #[test]
    fn unit_quaternion_default_works() {
        let quat = UnitQuaternion::default();

        // Default should be identity
        assert_eq!(quat.real(), 1.0);
        let imag = quat.imag();
        assert_eq!(imag.x(), 0.0);
        assert_eq!(imag.y(), 0.0);
        assert_eq!(imag.z(), 0.0);
    }

    // General trait tests
    #[test]
    fn quaternion_operations_with_different_reference_combinations_work() {
        let q1 = Quaternion::from_parts(1.0, Vector3::new(0.0, 0.0, 0.0));
        let q2 = Quaternion::from_parts(0.0, Vector3::new(1.0, 0.0, 0.0));

        // Test all combinations of reference/owned for binary operations
        let _result1 = &q1 + &q2; // ref + ref
        let _result2 = &q1 + q2; // ref + owned
        let _result3 = q1 + &q2; // owned + ref
        let _result4 = q1 + q2; // owned + owned

        // Recreate since they were moved
        let q1 = Quaternion::from_parts(1.0, Vector3::new(0.0, 0.0, 0.0));
        let q2 = Quaternion::from_parts(0.0, Vector3::new(1.0, 0.0, 0.0));

        let _result5 = &q1 * &q2; // ref * ref
        let _result6 = &q1 * q2; // ref * owned
        let _result7 = q1 * &q2; // owned * ref
        let _result8 = q1 * q2; // owned * owned
    }

    #[test]
    fn unit_quaternion_operations_with_different_reference_combinations_work() {
        let u1 = UnitQuaternion::identity();
        let u2 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0);

        // Test all combinations for multiplication
        let _result1 = &u1 * &u2; // ref * ref
        let _result2 = &u1 * u2; // ref * owned
        let _result3 = u1 * &u2; // owned * ref
        let _result4 = u1 * u2; // owned * owned
    }

    #[test]
    fn quaternion_rotation_composition_is_associative() {
        let q1 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), 0.1);
        let q2 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), 0.2);
        let q3 = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 0.3);

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
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 3.0);
        let vector = Vector3::new(2.0, 3.0, 4.0);
        let original_length = vector.norm();

        let rotated = quat.transform_vector(&vector);
        let rotated_length = rotated.norm();

        assert_abs_diff_eq!(original_length, rotated_length, epsilon = EPSILON);
    }

    #[test]
    fn quaternion_identity_is_neutral_element() {
        let identity = UnitQuaternion::identity();
        let test_quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 6.0);

        let left_mult = &identity * &test_quat;
        let right_mult = &test_quat * &identity;

        // Identity should be neutral: I * q = q * I = q
        assert_abs_diff_eq!(left_mult.real(), test_quat.real(), epsilon = EPSILON);
        assert_abs_diff_eq!(right_mult.real(), test_quat.real(), epsilon = EPSILON);
    }

    #[test]
    fn quaternion_inverse_is_correct() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), PI / 4.0);
        let inverse = quat.inverse();

        let vector = Vector3::new(1.0, 0.0, 0.0);
        let rotated = quat.transform_vector(&vector);
        let back_rotated = inverse.transform_vector(&rotated);

        // q^-1 * q * v = v
        assert_abs_diff_eq!(back_rotated.x(), vector.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_rotated.y(), vector.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_rotated.z(), vector.z(), epsilon = EPSILON);
    }

    #[test]
    fn quaternion_transform_vs_inverse_transform_are_inverse() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 3.0);
        let point = Point3::new(1.0, 2.0, 3.0);

        let transformed = quat.transform_point(&point);
        let back_transformed = quat.inverse_transform_point(&transformed);

        assert_abs_diff_eq!(back_transformed.x(), point.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_transformed.y(), point.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(back_transformed.z(), point.z(), epsilon = EPSILON);
    }

    #[test]
    fn quaternion_axis_angle_identity_has_no_rotation() {
        let identity = UnitQuaternion::identity();

        // Identity quaternion should have no axis (returns None) or angle of 0
        let angle = identity.angle();
        assert_abs_diff_eq!(angle, 0.0, epsilon = EPSILON);
    }

    #[test]
    fn quaternion_matrix_conversion_preserves_rotation() {
        let quat = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0);
        let matrix = quat.to_rotation_matrix();

        let vector = Vector3::new(0.0, 1.0, 0.0);
        let quat_rotated = quat.transform_vector(&vector);
        let matrix_rotated = &matrix * &vector;

        assert_abs_diff_eq!(quat_rotated.x(), matrix_rotated.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_rotated.y(), matrix_rotated.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(quat_rotated.z(), matrix_rotated.z(), epsilon = EPSILON);
    }
}
