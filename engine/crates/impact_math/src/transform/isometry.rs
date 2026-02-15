//! Isometry transforms.

use crate::{
    matrix::Matrix4,
    point::Point3,
    quaternion::{UnitQuaternion, UnitQuaternionC},
    vector::{UnitVector3, Vector3, Vector3C},
};
use bytemuck::{Pod, Zeroable};

/// A transform consisting of a rotation followed by a translation.
///
/// The rotation quaternion and translation vector are stored in 128-bit SIMD
/// registers for efficient computation. That leads to an extra 4 bytes in size
/// (due to the padded vector) and 16-byte alignment. For cache-friendly
/// storage, prefer the compact 4-byte aligned [`Isometry3C`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Isometry3 {
    rotation: UnitQuaternion,
    translation: Vector3,
}

/// A transform consisting of a rotation followed by a translation. This is the
/// "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Isometry3`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Isometry3C {
    rotation: UnitQuaternionC,
    translation: Vector3C,
}

impl Isometry3 {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            rotation: UnitQuaternion::identity(),
            translation: Vector3::zeros(),
        }
    }

    /// Creates the isometry transform consisting of the given rotation and
    /// translation.
    #[inline]
    pub const fn from_parts(translation: Vector3, rotation: UnitQuaternion) -> Self {
        Self {
            rotation,
            translation,
        }
    }

    /// Creates the isometry transform consisting of the given translation and
    /// no rotation.
    #[inline]
    pub const fn from_translation(translation: Vector3) -> Self {
        Self::from_parts(translation, UnitQuaternion::identity())
    }

    /// Creates the isometry transform consisting of the given rotation and
    /// no translation.
    #[inline]
    pub const fn from_rotation(rotation: UnitQuaternion) -> Self {
        Self::from_parts(Vector3::zeros(), rotation)
    }

    /// Creates the isometry transform corresponding to applying the given
    /// translation before the rotation.
    #[inline]
    pub fn from_rotated_translation(translation: Vector3, rotation: UnitQuaternion) -> Self {
        Self::from_parts(rotation.rotate_vector(&translation), rotation)
    }

    /// The translational part of the transform.
    #[inline]
    pub const fn translation(&self) -> &Vector3 {
        &self.translation
    }

    /// The rotational part of the transform.
    #[inline]
    pub const fn rotation(&self) -> &UnitQuaternion {
        &self.rotation
    }

    /// Returns the transform where the given translation is applied after this
    /// transform.
    #[inline]
    pub fn translated(&self, translation: &Vector3) -> Self {
        Self::from_parts(self.translation + translation, self.rotation)
    }

    /// Returns the transform where the given rotation is applied after this
    /// transform.
    #[inline]
    pub fn rotated(&self, rotation: &UnitQuaternion) -> Self {
        Self::from_parts(
            rotation.rotate_vector(&self.translation),
            rotation * self.rotation,
        )
    }

    /// Returns the transform where the given translation is applied before this
    /// transform.
    #[inline]
    pub fn applied_to_translation(&self, translation: &Vector3) -> Self {
        Self::from_parts(
            self.rotation.rotate_vector(translation) + self.translation,
            self.rotation,
        )
    }

    /// Returns the transform where the given rotation is applied before this
    /// transform.
    #[inline]
    pub fn applied_to_rotation(&self, rotation: &UnitQuaternion) -> Self {
        Self::from_parts(self.translation, self.rotation * rotation)
    }

    /// Computes the inverse of this transform.
    #[inline]
    pub fn inverted(&self) -> Self {
        let inverse_rotation = self.rotation.inverse();
        Self::from_parts(
            -inverse_rotation.rotate_vector(&self.translation),
            inverse_rotation,
        )
    }

    /// Converts the transform to a 4x4 homogeneous matrix.
    #[inline]
    pub fn to_matrix(&self) -> Matrix4 {
        let mut m = self.rotation.to_homogeneous_matrix();
        m.translate_transform(&self.translation);
        m
    }

    /// Applies the transform to the given point.
    #[inline]
    pub fn transform_point(&self, point: &Point3) -> Point3 {
        self.rotation.rotate_point(point) + self.translation
    }

    /// Applies the transform to the given vector. The translation part of the
    /// transform is not applied to vectors.
    #[inline]
    pub fn transform_vector(&self, vector: &Vector3) -> Vector3 {
        self.rotation.rotate_vector(vector)
    }

    /// Applies the transform to the given unit vector. The translation part of
    /// the transform is not applied to vectors.
    #[inline]
    pub fn transform_unit_vector(&self, vector: &UnitVector3) -> UnitVector3 {
        self.rotation.rotate_unit_vector(vector)
    }

    /// Applies the inverse of this transform to the given point. For a single
    /// transformation, this is more efficient than explicitly inverting the
    /// transform and then applying it.
    #[inline]
    pub fn inverse_transform_point(&self, point: &Point3) -> Point3 {
        self.rotation
            .inverse()
            .rotate_point(&(point - self.translation))
    }

    /// Applies the inverse of this transform to the given vector. For a single
    /// transformation, this is more efficient than explicitly inverting the
    /// transform and then applying it. The translation part of the transform is
    /// not applied to vectors.
    #[inline]
    pub fn inverse_transform_vector(&self, vector: &Vector3) -> Vector3 {
        self.rotation.inverse().rotate_vector(vector)
    }

    /// Applies the inverse of this transform to the given unit vector. For a
    /// single transformation, this is more efficient than explicitly inverting
    /// the transform and then applying it. The translation part of the
    /// transform is not applied to vectors.
    #[inline]
    pub fn inverse_transform_unit_vector(&self, vector: &UnitVector3) -> UnitVector3 {
        self.rotation.inverse().rotate_unit_vector(vector)
    }

    /// Converts the transform to the 4-byte aligned cache-friendly
    /// [`Isometry3C`].
    #[inline]
    pub fn compact(&self) -> Isometry3C {
        Isometry3C::from_parts(self.translation().compact(), self.rotation().compact())
    }
}

impl_binop!(Mul, mul, Isometry3, Isometry3, Isometry3, |a, b| {
    Isometry3::from_parts(
        a.rotation.rotate_vector(&b.translation) + a.translation,
        a.rotation * b.rotation,
    )
});

impl_abs_diff_eq!(Isometry3, |a, b, epsilon| {
    a.rotation.abs_diff_eq(&b.rotation, epsilon)
        && a.translation.abs_diff_eq(&b.translation, epsilon)
});

impl_relative_eq!(Isometry3, |a, b, epsilon, max_relative| {
    a.rotation.relative_eq(&b.rotation, epsilon, max_relative)
        && a.translation
            .relative_eq(&b.translation, epsilon, max_relative)
});

impl Isometry3C {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            rotation: UnitQuaternionC::identity(),
            translation: Vector3C::zeros(),
        }
    }

    /// Creates the isometry transform consisting of the given rotation and
    /// translation.
    #[inline]
    pub const fn from_parts(translation: Vector3C, rotation: UnitQuaternionC) -> Self {
        Self {
            rotation,
            translation,
        }
    }

    /// Creates the isometry transform consisting of the given translation and
    /// no rotation.
    #[inline]
    pub const fn from_translation(translation: Vector3C) -> Self {
        Self::from_parts(translation, UnitQuaternionC::identity())
    }

    /// Creates the isometry transform consisting of the given rotation and
    /// no translation.
    #[inline]
    pub const fn from_rotation(rotation: UnitQuaternionC) -> Self {
        Self::from_parts(Vector3C::zeros(), rotation)
    }

    /// The translational part of the transform.
    #[inline]
    pub const fn translation(&self) -> &Vector3C {
        &self.translation
    }

    /// The rotational part of the transform.
    #[inline]
    pub const fn rotation(&self) -> &UnitQuaternionC {
        &self.rotation
    }

    /// Converts the transform to the 16-byte aligned SIMD-friendly
    /// [`Isometry3`].
    #[inline]
    pub fn aligned(&self) -> Isometry3 {
        Isometry3::from_parts(self.translation().aligned(), self.rotation().aligned())
    }
}

impl_abs_diff_eq!(Isometry3C, |a, b, epsilon| {
    a.rotation.abs_diff_eq(&b.rotation, epsilon)
        && a.translation.abs_diff_eq(&b.translation, epsilon)
});

impl_relative_eq!(Isometry3C, |a, b, epsilon, max_relative| {
    a.rotation.relative_eq(&b.rotation, epsilon, max_relative)
        && a.translation
            .relative_eq(&b.translation, epsilon, max_relative)
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::f32::PI;
    use Vector3C;
    use approx::assert_abs_diff_eq;

    const EPSILON: f32 = 1e-6;
    const TRANSLATION_1: Vector3 = Vector3::new(1.0, 2.0, 3.0);
    const TRANSLATION_2: Vector3 = Vector3::new(4.0, 5.0, 6.0);
    const TRANSLATION_3: Vector3C = Vector3C::new(1.5, 2.5, 3.5);

    fn rotation_90_z() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0)
    }

    fn rotation_45_x() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0)
    }

    fn rotation_90_z_unaligned() -> UnitQuaternionC {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0).compact()
    }

    // === Isometry3 Tests (SIMD-aligned) ===

    #[test]
    fn isometry3_from_rotated_translation_applies_rotation_to_translation() {
        let translation = Vector3::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let iso = Isometry3::from_rotated_translation(translation, rotation);

        let expected_translation = rotation.rotate_vector(&translation);
        assert_abs_diff_eq!(*iso.translation(), expected_translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*iso.rotation(), rotation, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_translated_adds_to_existing_translation() {
        let iso = Isometry3::from_translation(TRANSLATION_1);
        let translated = iso.translated(&TRANSLATION_2);

        assert_eq!(*translated.translation(), TRANSLATION_1 + TRANSLATION_2);
    }

    #[test]
    fn isometry3_rotated_composes_rotations() {
        let rotation1 = rotation_90_z();
        let rotation2 = rotation_45_x();
        let iso = Isometry3::from_rotation(rotation1);
        let rotated = iso.rotated(&rotation2);

        assert_abs_diff_eq!(
            *rotated.rotation(),
            rotation2 * rotation1,
            epsilon = EPSILON
        );
    }

    #[test]
    fn isometry3_applied_to_translation_transforms_and_adds() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let additional_translation = TRANSLATION_2;
        let result = iso.applied_to_translation(&additional_translation);

        // Should transform the additional translation and add to existing
        let transformed_translation = iso.transform_vector(&additional_translation);
        let expected_translation = TRANSLATION_1 + transformed_translation;
        assert_abs_diff_eq!(
            result.translation(),
            &expected_translation,
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*result.rotation(), *iso.rotation(), epsilon = EPSILON);
    }

    #[test]
    fn isometry3_applied_to_rotation_composes_in_correct_order() {
        let rotation1 = rotation_90_z();
        let rotation2 = rotation_45_x();
        let iso = Isometry3::from_rotation(rotation1);
        let result = iso.applied_to_rotation(&rotation2);

        assert_abs_diff_eq!(*result.rotation(), rotation1 * rotation2, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_multiplied_by_inverse_gives_identity() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let inverted = iso.inverted();
        let result = iso * inverted;

        assert_abs_diff_eq!(result, Isometry3::identity(), epsilon = EPSILON);
    }

    #[test]
    fn isometry3_inverse_multiplied_gives_identity() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let inverted = iso.inverted();

        assert_abs_diff_eq!(inverted * iso, Isometry3::identity(), epsilon = EPSILON);
    }

    #[test]
    fn isometry3_to_matrix_produces_correct_transform() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let sim = Isometry3::from_parts(translation, rotation);
        let matrix = sim.to_matrix();

        // Test by transforming a point
        let point = Point3::new(1.0, 0.0, 0.0);
        let sim_transformed = sim.transform_point(&point);
        let matrix_transformed = matrix.transform_point(&point);

        assert_abs_diff_eq!(
            sim_transformed.x(),
            matrix_transformed.x(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            sim_transformed.y(),
            matrix_transformed.y(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim_transformed, matrix_transformed, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_transform_point_rotates_then_translates() {
        let point = Point3::new(1.0, 0.0, 0.0);
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let iso = Isometry3::from_parts(translation, rotation);
        let transformed = iso.transform_point(&point);

        // First rotate, then translate
        let rotated_coords = rotation.rotate_vector(point.as_vector());
        let expected = Point3::from(rotated_coords + translation);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_transform_vector_ignores_translation() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let transformed = iso.transform_vector(&vector);

        assert_abs_diff_eq!(
            transformed,
            rotation_90_z().rotate_vector(&vector),
            epsilon = EPSILON
        );
    }

    #[test]
    fn isometry3_inverse_transform_point_undoes_transform() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let transformed = iso.transform_point(&point);
        let back = iso.inverse_transform_point(&transformed);

        assert_abs_diff_eq!(back, point, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_inverse_transform_vector_undoes_transform() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let roundtrip = iso.inverse_transform_vector(&iso.transform_vector(&vector));

        assert_abs_diff_eq!(roundtrip, vector, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_inverse_transform_unit_vector_undoes_transform() {
        let unit_vector = UnitVector3::unit_x();
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let roundtrip = iso.inverse_transform_unit_vector(&iso.transform_unit_vector(&unit_vector));

        assert_abs_diff_eq!(roundtrip, unit_vector, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_multiplication_is_associative() {
        let iso1 = Isometry3::from_translation(TRANSLATION_1);
        let iso2 = Isometry3::from_rotation(rotation_90_z());
        let iso3 = Isometry3::from_translation(TRANSLATION_2);

        let result1 = (iso1 * iso2) * iso3;
        let result2 = iso1 * (iso2 * iso3);

        assert_abs_diff_eq!(result1, result2, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_multiplication_composes_transformations_correctly() {
        let iso1 = Isometry3::from_translation(TRANSLATION_1);
        let iso2 = Isometry3::from_rotation(rotation_90_z());
        let iso3 = Isometry3::from_translation(TRANSLATION_2);
        let composed = iso3 * iso2 * iso1;

        let point = Point3::new(1.0, 0.0, 0.0);
        let composed_result = composed.transform_point(&point);
        let step_by_step =
            iso3.transform_point(&iso2.transform_point(&iso1.transform_point(&point)));

        assert_abs_diff_eq!(composed_result, step_by_step, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_preserves_distances() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let point1 = Point3::new(0.0, 0.0, 0.0);
        let point2 = Point3::new(1.0, 1.0, 1.0);

        let original_distance = (point2.as_vector() - point1.as_vector()).norm();

        let transformed1 = iso.transform_point(&point1);
        let transformed2 = iso.transform_point(&point2);
        let transformed_distance = (transformed2.as_vector() - transformed1.as_vector()).norm();

        assert_abs_diff_eq!(transformed_distance, original_distance, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_preserves_angles() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_45_x());
        let origin = Point3::new(0.0, 0.0, 0.0);
        let point1 = Point3::new(1.0, 0.0, 0.0);
        let point2 = Point3::new(0.0, 1.0, 0.0);

        let vec1 = point1.as_vector() - origin.as_vector();
        let vec2 = point2.as_vector() - origin.as_vector();
        let original_angle = vec1.dot(&vec2) / (vec1.norm() * vec2.norm());

        let transformed_vec1 =
            iso.transform_point(&point1).as_vector() - iso.transform_point(&origin).as_vector();
        let transformed_vec2 =
            iso.transform_point(&point2).as_vector() - iso.transform_point(&origin).as_vector();
        let transformed_angle = transformed_vec1.dot(&transformed_vec2)
            / (transformed_vec1.norm() * transformed_vec2.norm());

        assert_abs_diff_eq!(transformed_angle, original_angle, epsilon = EPSILON);
    }

    #[test]
    fn isometry3_composing_many_transformations_accumulates_correctly() {
        let mut iso = Isometry3::identity();
        let small_translation = Vector3::new(0.001, 0.001, 0.001);
        let small_rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 0.1);

        for _ in 0..100 {
            iso = iso.translated(&small_translation).rotated(&small_rotation);
        }

        assert!(iso.translation().norm() < 1.0);
        assert!(iso.rotation().angle() < 2.0 * PI);
    }

    // === Isometry3C Tests (compact) ===

    #[test]
    fn converting_isometry3p_to_aligned_and_back_preserves_data() {
        let translation = TRANSLATION_3;
        let rotation = rotation_90_z_unaligned();
        let iso = Isometry3C::from_parts(translation, rotation);
        let roundtrip = iso.aligned().compact();

        assert_abs_diff_eq!(*roundtrip.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*roundtrip.rotation(), rotation, epsilon = EPSILON);
    }
}
