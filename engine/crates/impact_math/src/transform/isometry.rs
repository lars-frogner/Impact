//! Isometry transforms.

use crate::{
    point::Point3,
    quaternion::{UnitQuaternion, UnitQuaternionP},
    vector::{UnitVector3, Vector3, Vector3P},
};
use bytemuck::{Pod, Zeroable};

/// A transform consisting of a rotation followed by a translation.
///
/// The rotation quaternion and translation vector are stored in 128-bit SIMD
/// registers for efficient computation. That leads to an extra 4 bytes in size
/// (due to the padded vector) and 16-byte alignment. For cache-friendly
/// storage, prefer the packed 4-byte aligned [`Isometry3P`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Isometry3 {
    rotation: UnitQuaternion,
    translation: Vector3,
}

/// A transform consisting of a rotation followed by a translation. This is the
/// "packed" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Isometry3`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Isometry3P {
    rotation: UnitQuaternionP,
    translation: Vector3P,
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
    /// [`Isometry3P`].
    #[inline]
    pub fn pack(&self) -> Isometry3P {
        Isometry3P::from_parts(self.translation().pack(), self.rotation().pack())
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

impl Isometry3P {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            rotation: UnitQuaternionP::identity(),
            translation: Vector3P::zeros(),
        }
    }

    /// Creates the isometry transform consisting of the given rotation and
    /// translation.
    #[inline]
    pub const fn from_parts(translation: Vector3P, rotation: UnitQuaternionP) -> Self {
        Self {
            rotation,
            translation,
        }
    }

    /// Creates the isometry transform consisting of the given translation and
    /// no rotation.
    #[inline]
    pub const fn from_translation(translation: Vector3P) -> Self {
        Self::from_parts(translation, UnitQuaternionP::identity())
    }

    /// Creates the isometry transform consisting of the given rotation and
    /// no translation.
    #[inline]
    pub const fn from_rotation(rotation: UnitQuaternionP) -> Self {
        Self::from_parts(Vector3P::zeros(), rotation)
    }

    /// The translational part of the transform.
    #[inline]
    pub const fn translation(&self) -> &Vector3P {
        &self.translation
    }

    /// The rotational part of the transform.
    #[inline]
    pub const fn rotation(&self) -> &UnitQuaternionP {
        &self.rotation
    }

    /// Converts the transform to the 16-byte aligned SIMD-friendly
    /// [`Isometry3`].
    #[inline]
    pub fn unpack(&self) -> Isometry3 {
        Isometry3::from_parts(self.translation().unpack(), self.rotation().unpack())
    }
}

impl_abs_diff_eq!(Isometry3P, |a, b, epsilon| {
    a.rotation.abs_diff_eq(&b.rotation, epsilon)
        && a.translation.abs_diff_eq(&b.translation, epsilon)
});

impl_relative_eq!(Isometry3P, |a, b, epsilon, max_relative| {
    a.rotation.relative_eq(&b.rotation, epsilon, max_relative)
        && a.translation
            .relative_eq(&b.translation, epsilon, max_relative)
});

#[cfg(test)]
mod tests {
    use super::*;
    use Vector3P;
    use approx::assert_abs_diff_eq;
    use std::f32::consts::PI;

    // Test constants
    const EPSILON: f32 = 1e-6;
    const TRANSLATION_1: Vector3 = Vector3::new(1.0, 2.0, 3.0);
    const TRANSLATION_2: Vector3 = Vector3::new(4.0, 5.0, 6.0);
    const TRANSLATION_3: Vector3P = Vector3P::new(1.5, 2.5, 3.5);

    fn rotation_90_z() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0)
    }

    fn rotation_45_x() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0)
    }

    fn rotation_90_z_unaligned() -> UnitQuaternionP {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0).pack()
    }

    // Isometry3P tests
    #[test]
    fn creating_identity_isometry3_gives_zero_translation_identity_rotation() {
        let iso = Isometry3P::identity();

        assert_abs_diff_eq!(*iso.translation(), Vector3P::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *iso.rotation(),
            UnitQuaternionP::identity(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn creating_isometry3_from_parts_stores_translation_and_rotation() {
        let translation = TRANSLATION_3;
        let rotation = rotation_90_z_unaligned();
        let iso = Isometry3P::from_parts(translation, rotation);

        assert_abs_diff_eq!(*iso.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*iso.rotation(), rotation, epsilon = EPSILON);
    }

    #[test]
    fn creating_isometry3_from_translation_has_identity_rotation() {
        let translation = TRANSLATION_3;
        let iso = Isometry3P::from_translation(translation);

        assert_abs_diff_eq!(*iso.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *iso.rotation(),
            UnitQuaternionP::identity(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn creating_isometry3_from_rotation_has_zero_translation() {
        let rotation = rotation_90_z_unaligned();
        let iso = Isometry3P::from_rotation(rotation);

        assert_abs_diff_eq!(*iso.translation(), Vector3P::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*iso.rotation(), rotation, epsilon = EPSILON);
    }

    #[test]
    fn converting_isometry3_to_aligned_works() {
        let translation = TRANSLATION_3;
        let rotation = rotation_90_z_unaligned();
        let iso = Isometry3P::from_parts(translation, rotation);
        let aligned = iso.unpack();

        assert_abs_diff_eq!(
            *aligned.translation(),
            translation.unpack(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*aligned.rotation(), rotation.unpack(), epsilon = EPSILON);
    }

    // Isometry3 tests (aligned version)
    #[test]
    fn creating_identity_isometry3a_gives_zero_translation_identity_rotation() {
        let iso = Isometry3::identity();

        assert_abs_diff_eq!(*iso.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *iso.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn identity_isometry3a_equals_default() {
        let identity = Isometry3::identity();
        let default = Isometry3::default();

        assert_abs_diff_eq!(identity, default, epsilon = EPSILON);
    }

    // Construction tests
    #[test]
    fn creating_isometry3a_from_parts_stores_translation_and_rotation() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let iso = Isometry3::from_parts(translation, rotation);

        assert_abs_diff_eq!(*iso.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*iso.rotation(), rotation, epsilon = EPSILON);
    }

    #[test]
    fn creating_isometry3a_from_translation_has_identity_rotation() {
        let translation = TRANSLATION_1;
        let iso = Isometry3::from_translation(translation);

        assert_abs_diff_eq!(*iso.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *iso.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn creating_isometry3a_from_rotation_has_zero_translation() {
        let rotation = rotation_90_z();
        let iso = Isometry3::from_rotation(rotation);

        assert_abs_diff_eq!(*iso.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*iso.rotation(), rotation, epsilon = EPSILON);
    }

    #[test]
    fn creating_isometry3a_from_rotated_translation_applies_rotation_to_translation() {
        let translation = Vector3::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let iso = Isometry3::from_rotated_translation(translation, rotation);

        let expected_translation = rotation.rotate_vector(&translation);
        assert_abs_diff_eq!(*iso.translation(), expected_translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*iso.rotation(), rotation, epsilon = EPSILON);
    }

    // Transformation composition tests
    #[test]
    fn translating_isometry3a_adds_translation() {
        let iso = Isometry3::from_translation(TRANSLATION_1);
        let additional_translation = TRANSLATION_2;
        let translated = iso.translated(&additional_translation);

        let expected_translation = TRANSLATION_1 + additional_translation;
        assert_abs_diff_eq!(
            translated.translation(),
            &expected_translation,
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*translated.rotation(), *iso.rotation(), epsilon = EPSILON);
    }

    #[test]
    fn rotating_isometry3a_composes_rotations() {
        let rotation1 = rotation_90_z();
        let rotation2 = rotation_45_x();
        let iso = Isometry3::from_rotation(rotation1);
        let rotated = iso.rotated(&rotation2);

        let expected_rotation = rotation2 * rotation1;
        assert_abs_diff_eq!(*rotated.rotation(), expected_rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(rotated.translation(), iso.translation(), epsilon = EPSILON);
    }

    #[test]
    fn applying_isometry3a_to_translation_works() {
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
    fn applying_isometry3a_to_rotation_composes_rotations_in_order() {
        let rotation1 = rotation_90_z();
        let rotation2 = rotation_45_x();
        let iso = Isometry3::from_rotation(rotation1);
        let result = iso.applied_to_rotation(&rotation2);

        let expected_rotation = rotation1 * rotation2;
        assert_abs_diff_eq!(*result.rotation(), expected_rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(result.translation(), iso.translation(), epsilon = EPSILON);
    }

    // Inversion tests
    #[test]
    fn inverting_identity_isometry3a_gives_identity() {
        let identity = Isometry3::identity();
        let inverted = identity.inverted();

        assert_abs_diff_eq!(inverted, identity, epsilon = EPSILON);
    }

    #[test]
    fn inverting_isometry3a_with_translation_gives_negative_translation() {
        let translation = TRANSLATION_1;
        let iso = Isometry3::from_translation(translation);
        let inverted = iso.inverted();

        assert_abs_diff_eq!(*inverted.translation(), -translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *inverted.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn inverting_isometry3a_with_rotation_gives_inverse_rotation() {
        let rotation = rotation_90_z();
        let iso = Isometry3::from_rotation(rotation);
        let inverted = iso.inverted();

        assert_abs_diff_eq!(*inverted.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*inverted.rotation(), rotation.inverse(), epsilon = EPSILON);
    }

    #[test]
    fn multiplying_isometry3a_with_inverse_gives_identity() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let inverted = iso.inverted();
        let result = iso * inverted;

        assert_abs_diff_eq!(result, Isometry3::identity(), epsilon = EPSILON);
    }

    #[test]
    fn multiplying_inverse_with_isometry3a_gives_identity() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let inverted = iso.inverted();
        let result = inverted * iso;

        assert_abs_diff_eq!(result, Isometry3::identity(), epsilon = EPSILON);
    }

    // Point transformation tests
    #[test]
    fn transforming_point_with_identity_gives_same_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let identity = Isometry3::identity();
        let transformed = identity.transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_translation_adds_translation() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let iso = Isometry3::from_translation(translation);
        let transformed = iso.transform_point(&point);

        let expected = Point3::from(*point.as_vector() + translation);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_rotation_rotates_point() {
        let point = Point3::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let iso = Isometry3::from_rotation(rotation);
        let transformed = iso.transform_point(&point);

        let expected_coords = rotation.rotate_vector(point.as_vector());
        let expected = Point3::from(expected_coords);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_unit_vector_with_identity_gives_same_unit_vector() {
        let unit_vector = UnitVector3::unit_x();
        let identity = Isometry3::identity();
        let transformed = identity.transform_unit_vector(&unit_vector);

        assert_abs_diff_eq!(transformed, unit_vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_unit_vector_with_rotation_rotates_unit_vector() {
        let unit_vector = UnitVector3::unit_x();
        let rotation = rotation_90_z();
        let iso = Isometry3::from_rotation(rotation);
        let transformed = iso.transform_unit_vector(&unit_vector);

        let expected = rotation.rotate_unit_vector(&unit_vector);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_full_isometry_works() {
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

    // Vector transformation tests
    #[test]
    fn transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let identity = Isometry3::identity();
        let transformed = identity.transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_translation_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let iso = Isometry3::from_translation(translation);
        let transformed = iso.transform_vector(&vector);

        // Vectors should not be affected by translation
        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_rotation_rotates_vector() {
        let vector = Vector3::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let iso = Isometry3::from_rotation(rotation);
        let transformed = iso.transform_vector(&vector);

        let expected = rotation.rotate_vector(&vector);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    // Inverse transformation tests
    #[test]
    fn inverse_transforming_point_with_identity_gives_same_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let identity = Isometry3::identity();
        let transformed = identity.inverse_transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let transformed = iso.transform_point(&point);
        let back = iso.inverse_transform_point(&transformed);

        assert_abs_diff_eq!(back, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let identity = Isometry3::identity();
        let transformed = identity.inverse_transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let transformed = iso.transform_vector(&vector);
        let back = iso.inverse_transform_vector(&transformed);

        assert_abs_diff_eq!(back, vector, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transforming_unit_vector_with_identity_gives_same_unit_vector() {
        let unit_vector = UnitVector3::unit_x();
        let identity = Isometry3::identity();
        let transformed = identity.inverse_transform_unit_vector(&unit_vector);

        assert_abs_diff_eq!(transformed, unit_vector, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_unit_vector() {
        let unit_vector = UnitVector3::unit_x();
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let transformed = iso.transform_unit_vector(&unit_vector);
        let back = iso.inverse_transform_unit_vector(&transformed);

        assert_abs_diff_eq!(back, unit_vector, epsilon = EPSILON);
    }

    // Multiplication tests
    #[test]
    fn multiplying_by_identity_gives_same_isometry() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
        let identity = Isometry3::identity();

        let result1 = iso * identity;
        let result2 = identity * iso;

        assert_abs_diff_eq!(result1, iso, epsilon = EPSILON);
        assert_abs_diff_eq!(result2, iso, epsilon = EPSILON);
    }

    // Conversion tests
    #[test]
    fn converting_isometry3_to_isometry3a_preserves_values() {
        let translation = TRANSLATION_3;
        let rotation = rotation_90_z_unaligned();
        let iso3 = Isometry3P::from_parts(translation, rotation);
        let iso3a = iso3.unpack();

        assert_abs_diff_eq!(
            *iso3a.translation(),
            translation.unpack(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*iso3a.rotation(), rotation.unpack(), epsilon = EPSILON);
    }

    #[test]
    fn converting_isometry3a_to_isometry3_preserves_values() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let iso3a = Isometry3::from_parts(translation, rotation);
        let iso3 = iso3a.pack();

        assert_abs_diff_eq!(*iso3.translation(), translation.pack(), epsilon = EPSILON);
        assert_abs_diff_eq!(*iso3.rotation(), rotation.pack(), epsilon = EPSILON);
    }

    #[test]
    fn converting_isometry3a_to_unaligned_preserves_values() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let iso3a = Isometry3::from_parts(translation, rotation);
        let iso3 = iso3a.pack();

        assert_abs_diff_eq!(*iso3.translation(), translation.pack(), epsilon = EPSILON);
        assert_abs_diff_eq!(*iso3.rotation(), rotation.pack(), epsilon = EPSILON);
    }

    #[test]
    fn round_trip_conversion_preserves_values() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let original = Isometry3::from_parts(translation, rotation);
        let converted = original.pack().unpack();

        assert_abs_diff_eq!(converted, original, epsilon = EPSILON);
    }

    #[test]
    fn multiplying_isometry3a_is_associative() {
        let iso1 = Isometry3::from_translation(TRANSLATION_1);
        let iso2 = Isometry3::from_rotation(rotation_90_z());
        let iso3 = Isometry3::from_translation(TRANSLATION_2);

        let result1 = (iso1 * iso2) * iso3;
        let result2 = iso1 * (iso2 * iso3);

        assert_abs_diff_eq!(result1, result2, epsilon = EPSILON);
    }

    #[test]
    fn multiplying_isometry3a_composes_transformations_correctly() {
        let translation1 = TRANSLATION_1;
        let rotation = rotation_90_z();
        let translation2 = TRANSLATION_2;

        let iso1 = Isometry3::from_translation(translation1);
        let iso2 = Isometry3::from_rotation(rotation);
        let iso3 = Isometry3::from_translation(translation2);

        let composed = iso3 * iso2 * iso1;

        // Test on a point
        let point = Point3::new(1.0, 0.0, 0.0);
        let result1 = composed.transform_point(&point);

        // Apply transformations step by step
        let step1 = iso1.transform_point(&point);
        let step2 = iso2.transform_point(&step1);
        let step3 = iso3.transform_point(&step2);

        assert_abs_diff_eq!(result1, step3, epsilon = EPSILON);
    }

    // Property tests
    #[test]
    fn isometry3a_preserves_distances() {
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
    fn isometry3a_preserves_angles() {
        let iso = Isometry3::from_parts(TRANSLATION_1, rotation_45_x());
        let origin = Point3::new(0.0, 0.0, 0.0);
        let point1 = Point3::new(1.0, 0.0, 0.0);
        let point2 = Point3::new(0.0, 1.0, 0.0);

        let vec1 = point1.as_vector() - origin.as_vector();
        let vec2 = point2.as_vector() - origin.as_vector();
        let original_angle = vec1.dot(&vec2) / (vec1.norm() * vec2.norm());

        let transformed_origin = iso.transform_point(&origin);
        let transformed1 = iso.transform_point(&point1);
        let transformed2 = iso.transform_point(&point2);

        let transformed_vec1 = transformed1.as_vector() - transformed_origin.as_vector();
        let transformed_vec2 = transformed2.as_vector() - transformed_origin.as_vector();
        let transformed_angle = transformed_vec1.dot(&transformed_vec2)
            / (transformed_vec1.norm() * transformed_vec2.norm());

        assert_abs_diff_eq!(transformed_angle, original_angle, epsilon = EPSILON);
    }

    // Edge case tests
    #[test]
    fn creating_isometry3a_with_very_small_translations_works() {
        let small_translation = Vector3::new(1e-10, 1e-10, 1e-10);
        let iso = Isometry3::from_translation(small_translation);

        assert_abs_diff_eq!(iso.translation(), &small_translation, epsilon = 1e-12);
    }

    #[test]
    fn creating_isometry3a_with_very_small_rotations_works() {
        let small_angle = 1e-6;
        let small_rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), small_angle);
        let iso = Isometry3::from_rotation(small_rotation);

        assert_abs_diff_eq!(*iso.rotation(), small_rotation, epsilon = 1e-9);
    }

    #[test]
    fn creating_isometry3a_with_large_translations_works() {
        let large_translation = Vector3::new(1e6, 1e6, 1e6);
        let iso = Isometry3::from_translation(large_translation);

        assert_abs_diff_eq!(iso.translation(), &large_translation, epsilon = 1e-3);
    }

    #[test]
    fn creating_isometry3a_with_multiple_full_rotations_works() {
        let full_rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 4.0 * PI);
        let iso = Isometry3::from_rotation(full_rotation);

        // Should be equivalent to identity (within floating point precision)
        let identity_rot = UnitQuaternion::identity();
        assert_abs_diff_eq!(*iso.rotation(), identity_rot, epsilon = 1e-5);
    }

    #[test]
    fn composing_many_small_isometry3a_transformations_works() {
        let mut iso = Isometry3::identity();
        let small_translation = Vector3::new(0.001, 0.001, 0.001);
        let small_rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 0.1);

        for _ in 0..100 {
            iso = iso.translated(&small_translation);
            iso = iso.rotated(&small_rotation);
        }

        // Should accumulate to reasonable values
        assert!(iso.translation().norm() < 1.0);
        assert!(iso.rotation().angle() < 2.0 * PI);
    }
}
