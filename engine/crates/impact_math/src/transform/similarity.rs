//! Similarity transforms.

use crate::{
    matrix::Matrix4,
    point::Point3,
    quaternion::{UnitQuaternion, UnitQuaternionP},
    transform::{Isometry3, Isometry3P},
    vector::{Vector3, Vector3P},
};
use bytemuck::{Pod, Zeroable};

/// A transform consisting of a uniform scaling and a rotation followed by a
/// translation.
///
/// The rotation quaternion and translation vector are stored in 128-bit SIMD
/// registers for efficient computation. That leads to an extra 16 bytes in size
/// (4 due to the padded vector and 12 due to padding after the scale factor)
/// and 16-byte alignment. For cache-friendly storage, prefer the packed 4-byte
/// aligned [`Similarity3P`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct Similarity3 {
    rotation: UnitQuaternion,
    translation: Vector3,
    scaling: f32,
}

/// A transform consisting of a uniform scaling and a rotation followed by a
/// translation. This is the "packed" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Similarity3`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct Similarity3P {
    rotation: UnitQuaternionP,
    translation: Vector3P,
    scaling: f32,
}

impl Similarity3 {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_parts(Vector3::zeros(), UnitQuaternion::identity(), 1.0)
    }

    /// Creates the similarity transform consisting of the given uniform
    /// scaling, rotation and translation.
    #[inline]
    pub const fn from_parts(translation: Vector3, rotation: UnitQuaternion, scaling: f32) -> Self {
        Self {
            rotation,
            translation,
            scaling,
        }
    }

    /// Creates the similarity transform corresponding to the given isometry
    /// transform (meaning a unit scale factor).
    #[inline]
    pub const fn from_isometry(isometry: Isometry3) -> Self {
        Self::from_parts(*isometry.translation(), *isometry.rotation(), 1.0)
    }

    /// Creates the similarity transform consisting of the given translation and
    /// no rotation or scaling.
    #[inline]
    pub const fn from_translation(translation: Vector3) -> Self {
        Self::from_parts(translation, UnitQuaternion::identity(), 1.0)
    }

    /// Creates the similarity transform consisting of the given rotation and
    /// no translation or scaling.
    #[inline]
    pub const fn from_rotation(rotation: UnitQuaternion) -> Self {
        Self::from_parts(Vector3::zeros(), rotation, 1.0)
    }

    /// Creates the similarity transform consisting of the given scaling and no
    /// translation or rotation.
    #[inline]
    pub const fn from_scaling(scaling: f32) -> Self {
        Self::from_parts(Vector3::zeros(), UnitQuaternion::identity(), scaling)
    }

    /// Creates the similarity transform corresponding to applying the given
    /// translation before the scaling.
    #[inline]
    pub fn from_scaled_translation(translation: Vector3, scaling: f32) -> Self {
        Self::from_parts(translation * scaling, UnitQuaternion::identity(), scaling)
    }

    /// Creates the similarity transform corresponding to applying the given
    /// rotation before the scaling.
    #[inline]
    pub fn from_scaled_rotation(rotation: UnitQuaternion, scaling: f32) -> Self {
        Self::from_rotation(rotation).scaled(scaling)
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

    /// The scaling part of the transform.
    #[inline]
    pub const fn scaling(&self) -> f32 {
        self.scaling
    }

    /// Returns the rotational and translational parts of the transform as an
    /// isometry.
    #[inline]
    pub const fn to_isometry(&self) -> Isometry3 {
        Isometry3::from_parts(self.translation, self.rotation)
    }

    /// Returns the transform where the given translation is applied after this
    /// transform.
    #[inline]
    pub fn translated(&self, translation: &Vector3) -> Self {
        Self::from_parts(self.translation + translation, self.rotation, self.scaling)
    }

    /// Returns the transform where the given rotation is applied after this
    /// transform.
    #[inline]
    pub fn rotated(&self, rotation: &UnitQuaternion) -> Self {
        Self::from_parts(
            rotation.rotate_vector(&self.translation),
            rotation * self.rotation,
            self.scaling,
        )
    }

    /// Returns the transform where the given scaling is applied after this
    /// transform.
    #[inline]
    pub fn scaled(&self, scaling: f32) -> Self {
        Self::from_parts(
            scaling * self.translation,
            self.rotation,
            scaling * self.scaling,
        )
    }

    /// Returns the transform where the given translation is applied before this
    /// transform.
    #[inline]
    pub fn applied_to_translation(&self, translation: &Vector3) -> Self {
        Self::from_parts(
            self.rotation.rotate_vector(&(self.scaling * translation)) + self.translation,
            self.rotation,
            self.scaling,
        )
    }

    /// Returns the transform where the given rotation is applied before this
    /// transform.
    #[inline]
    pub fn applied_to_rotation(&self, rotation: &UnitQuaternion) -> Self {
        Self::from_parts(self.translation, self.rotation * rotation, self.scaling)
    }

    /// Returns the transform where the given scaling is applied before this
    /// transform.
    #[inline]
    pub fn applied_to_scaling(&self, scaling: f32) -> Self {
        Self::from_parts(self.translation, self.rotation, self.scaling * scaling)
    }

    /// Computes the inverse of this transform. If the scaling is zero, the
    /// result will be non-finite.
    #[inline]
    pub fn inverted(&self) -> Self {
        let inverse_rotation = self.rotation.inverse();
        let inverse_scaling = self.scaling.recip();
        Self::from_parts(
            -inverse_rotation.rotate_vector(&(inverse_scaling * self.translation)),
            inverse_rotation,
            inverse_scaling,
        )
    }

    /// Converts the transform to a 4x4 homogeneous matrix.
    #[inline]
    pub fn to_matrix(&self) -> Matrix4 {
        let mut m = self.rotation.to_homogeneous_matrix();
        m.scale_transform(self.scaling);
        m.translate_transform(&self.translation);
        m
    }

    /// Applies the transform to the given point.
    #[inline]
    pub fn transform_point(&self, point: &Point3) -> Point3 {
        self.rotation.rotate_point(&(self.scaling * point)) + self.translation
    }

    /// Applies the transform to the given vector. The translation part of the
    /// transform is not applied to vectors.
    #[inline]
    pub fn transform_vector(&self, vector: &Vector3) -> Vector3 {
        self.rotation.rotate_vector(&(self.scaling * vector))
    }

    /// Applies the inverse of this transform to the given point. For a single
    /// transformation, this is more efficient than explicitly inverting the
    /// transform and then applying it.
    #[inline]
    pub fn inverse_transform_point(&self, point: &Point3) -> Point3 {
        self.rotation
            .inverse()
            .rotate_point(&(point - self.translation))
            / self.scaling
    }

    /// Applies the inverse of this transform to the given vector. For a single
    /// transformation, this is more efficient than explicitly inverting the
    /// transform and then applying it. The translation part of the transform is
    /// not applied to vectors.
    #[inline]
    pub fn inverse_transform_vector(&self, vector: &Vector3) -> Vector3 {
        self.rotation.inverse().rotate_vector(vector) / self.scaling
    }

    /// Converts the transform to the 4-byte aligned cache-friendly
    /// [`Similarity3P`].
    #[inline]
    pub fn pack(&self) -> Similarity3P {
        Similarity3P::from_parts(
            self.translation().pack(),
            self.rotation().pack(),
            self.scaling(),
        )
    }
}

impl Default for Similarity3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl_binop!(Mul, mul, Similarity3, Isometry3, Similarity3, |a, b| {
    Similarity3::from_parts(
        a.rotation.rotate_vector(&(a.scaling * b.translation())) + a.translation,
        a.rotation * b.rotation(),
        a.scaling,
    )
});

impl_binop!(Mul, mul, Isometry3, Similarity3, Similarity3, |a, b| {
    Similarity3::from_parts(
        a.rotation().rotate_vector(&b.translation) + a.translation(),
        a.rotation() * b.rotation,
        b.scaling,
    )
});

impl_binop!(Mul, mul, Similarity3, Similarity3, Similarity3, |a, b| {
    Similarity3::from_parts(
        a.rotation.rotate_vector(&(a.scaling * b.translation)) + a.translation,
        a.rotation * b.rotation,
        a.scaling * b.scaling,
    )
});

impl_abs_diff_eq!(Similarity3, |a, b, epsilon| {
    a.rotation.abs_diff_eq(&b.rotation, epsilon)
        && a.translation.abs_diff_eq(&b.translation, epsilon)
        && a.scaling.abs_diff_eq(&b.scaling, epsilon)
});

impl_relative_eq!(Similarity3, |a, b, epsilon, max_relative| {
    a.rotation.relative_eq(&b.rotation, epsilon, max_relative)
        && a.translation
            .relative_eq(&b.translation, epsilon, max_relative)
        && a.scaling.relative_eq(&b.scaling, epsilon, max_relative)
});

impl Similarity3P {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_parts(Vector3P::zeros(), UnitQuaternionP::identity(), 1.0)
    }

    /// Creates the similarity transform consisting of the given uniform
    /// scaling, rotation and translation.
    #[inline]
    pub const fn from_parts(
        translation: Vector3P,
        rotation: UnitQuaternionP,
        scaling: f32,
    ) -> Self {
        Self {
            rotation,
            translation,
            scaling,
        }
    }

    /// Creates the similarity transform corresponding to the given isometry
    /// transform (meaning a unit scale factor).
    #[inline]
    pub const fn from_isometry(isometry: Isometry3P) -> Self {
        Self::from_parts(*isometry.translation(), *isometry.rotation(), 1.0)
    }

    /// Creates the similarity transform consisting of the given translation and
    /// no rotation or scaling.
    #[inline]
    pub const fn from_translation(translation: Vector3P) -> Self {
        Self::from_parts(translation, UnitQuaternionP::identity(), 1.0)
    }

    /// Creates the similarity transform consisting of the given rotation and
    /// no translation or scaling.
    #[inline]
    pub const fn from_rotation(rotation: UnitQuaternionP) -> Self {
        Self::from_parts(Vector3P::zeros(), rotation, 1.0)
    }

    /// Creates the similarity transform consisting of the given scaling and no
    /// translation or rotation.
    #[inline]
    pub const fn from_scaling(scaling: f32) -> Self {
        Self::from_parts(Vector3P::zeros(), UnitQuaternionP::identity(), scaling)
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

    /// The scaling part of the transform.
    #[inline]
    pub const fn scaling(&self) -> f32 {
        self.scaling
    }

    /// Returns the rotational and translational parts of the transform as an
    /// isometry.
    #[inline]
    pub const fn to_isometry(&self) -> Isometry3P {
        Isometry3P::from_parts(self.translation, self.rotation)
    }

    /// Converts the transform to the 16-byte aligned SIMD-friendly
    /// [`Similarity3`].
    #[inline]
    pub fn unpack(&self) -> Similarity3 {
        Similarity3::from_parts(
            self.translation().unpack(),
            self.rotation().unpack(),
            self.scaling(),
        )
    }
}

impl Default for Similarity3P {
    fn default() -> Self {
        Self::identity()
    }
}

impl_abs_diff_eq!(Similarity3P, |a, b, epsilon| {
    a.rotation.abs_diff_eq(&b.rotation, epsilon)
        && a.translation.abs_diff_eq(&b.translation, epsilon)
        && a.scaling.abs_diff_eq(&b.scaling, epsilon)
});

impl_relative_eq!(Similarity3P, |a, b, epsilon, max_relative| {
    a.rotation.relative_eq(&b.rotation, epsilon, max_relative)
        && a.translation
            .relative_eq(&b.translation, epsilon, max_relative)
        && a.scaling.relative_eq(&b.scaling, epsilon, max_relative)
});

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use crate::{
        matrix::Matrix4P,
        vector::{UnitVector3, Vector4P},
    };
    use approx::assert_abs_diff_eq;
    use std::f32::consts::PI;

    const EPSILON: f32 = 1e-6;
    const TRANSLATION_1: Vector3 = Vector3::new(1.0, 2.0, 3.0);
    const TRANSLATION_2: Vector3 = Vector3::new(4.0, 5.0, 6.0);

    fn rotation_90_z() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0)
    }

    fn rotation_45_x() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0)
    }

    // === Similarity3 Tests (SIMD-aligned) ===

    #[test]
    fn similarity3_from_scaled_translation_scales_translation() {
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let scaling = 2.0;
        let sim = Similarity3::from_scaled_translation(translation, scaling);

        assert_abs_diff_eq!(*sim.translation(), translation * scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_scaled_rotation_applies_scaling() {
        let rotation = rotation_90_z();
        let scaling = 3.0;
        let sim = Similarity3::from_scaled_rotation(rotation, scaling);

        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_translated_adds_to_existing_translation() {
        let sim = Similarity3::from_translation(TRANSLATION_1);
        let translated = sim.translated(&TRANSLATION_2);

        assert_abs_diff_eq!(
            *translated.translation(),
            TRANSLATION_1 + TRANSLATION_2,
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*translated.rotation(), *sim.rotation(), epsilon = EPSILON);
        assert_abs_diff_eq!(translated.scaling(), sim.scaling(), epsilon = EPSILON);
    }

    #[test]
    fn similarity3_rotated_composes_rotations() {
        let rotated = Similarity3::from_rotation(rotation_90_z()).rotated(&rotation_45_x());

        assert_abs_diff_eq!(
            *rotated.rotation(),
            rotation_45_x() * rotation_90_z(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity3_scaled_multiplies_scaling() {
        let scaled = Similarity3::from_scaling(2.0).scaled(3.0);

        assert_abs_diff_eq!(scaled.scaling(), 6.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_applied_to_translation_transforms_and_adds() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.0);
        let result = sim.applied_to_translation(&TRANSLATION_2);

        let expected = TRANSLATION_1 + sim.transform_vector(&TRANSLATION_2);
        assert_abs_diff_eq!(*result.translation(), expected, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_applied_to_rotation_composes_in_order() {
        let sim = Similarity3::from_rotation(rotation_90_z());
        let result = sim.applied_to_rotation(&rotation_45_x());

        assert_abs_diff_eq!(
            *result.rotation(),
            rotation_90_z() * rotation_45_x(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity3_applied_to_scaling_multiplies() {
        let result = Similarity3::from_scaling(2.0).applied_to_scaling(3.0);

        assert_abs_diff_eq!(result.scaling(), 6.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_multiplied_by_inverse_gives_identity() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let inverted = sim.inverted();
        let result = sim * inverted;

        assert_abs_diff_eq!(result, Similarity3::identity(), epsilon = EPSILON);
    }

    #[test]
    fn similarity3_inverse_multiplied_gives_identity() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);

        assert_abs_diff_eq!(
            sim.inverted() * sim,
            Similarity3::identity(),
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity3_to_matrix_produces_correct_transform() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.0;
        let sim = Similarity3::from_parts(translation, rotation, scaling);
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
    fn similarity3_to_isometry_drops_scaling() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.0);
        let isometry = sim.to_isometry();

        assert_abs_diff_eq!(*isometry.translation(), TRANSLATION_1, epsilon = EPSILON);
        assert_abs_diff_eq!(*isometry.rotation(), rotation_90_z(), epsilon = EPSILON);
    }

    #[test]
    fn similarity3_transform_point_applies_scale_rotate_translate() {
        let point = Point3::new(1.0, 0.0, 0.0);
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.0;
        let sim = Similarity3::from_parts(translation, rotation, scaling);
        let transformed = sim.transform_point(&point);

        let expected =
            Point3::from(rotation.rotate_vector(&(*point.as_vector() * scaling)) + translation);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_transform_vector_applies_scale_and_rotation() {
        let vector = Vector3::new(1.0, 0.0, 0.0);
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.0);
        let transformed = sim.transform_vector(&vector);

        let expected = rotation_90_z().rotate_vector(&vector) * 2.0;
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_inverse_transform_point_undoes_transform() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let roundtrip = sim.inverse_transform_point(&sim.transform_point(&point));

        assert_abs_diff_eq!(roundtrip, point, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_inverse_transform_vector_undoes_transform() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let roundtrip = sim.inverse_transform_vector(&sim.transform_vector(&vector));

        assert_abs_diff_eq!(roundtrip, vector, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_multiplied_by_isometry_composes_correctly() {
        let sim = Similarity3::from_scaling(2.0);
        let iso = Isometry3::from_translation(TRANSLATION_1);

        let result1 = &sim * &iso; // Similarity3 * Isometry3 -> Similarity3
        let result2 = &iso * &sim; // Isometry3 * Similarity3 -> Similarity3

        // Test by transforming a point
        let point = Point3::new(1.0, 0.0, 0.0);

        assert_abs_diff_eq!(
            result1.transform_point(&point),
            sim.transform_point(&iso.transform_point(&point)),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            result2.transform_point(&point),
            iso.transform_point(&sim.transform_point(&point)),
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity3_multiplication_composes_correctly() {
        let sim1 = Similarity3::from_scaling(2.0);
        let sim2 = Similarity3::from_translation(TRANSLATION_1);
        let point = Point3::new(1.0, 0.0, 0.0);

        assert_abs_diff_eq!(
            (&sim2 * &sim1).transform_point(&point),
            sim2.transform_point(&sim1.transform_point(&point)),
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity3_multiplication_is_associative() {
        let sim1 = Similarity3::from_translation(TRANSLATION_1);
        let sim2 = Similarity3::from_rotation(rotation_90_z());
        let sim3 = Similarity3::from_scaling(2.0);

        let result1 = (&sim1 * &sim2) * &sim3;
        let result2 = &sim1 * (&sim2 * &sim3);

        assert_abs_diff_eq!(result1, result2, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_scales_distances_uniformly() {
        let scaling = 3.0;
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_45_x(), scaling);

        let point1 = Point3::new(1.0, 2.0, 3.0);
        let point2 = Point3::new(4.0, 5.0, 6.0);

        let original_distance = (point2.as_vector() - point1.as_vector()).norm();

        let transformed_distance = (sim.transform_point(&point2).as_vector()
            - sim.transform_point(&point1).as_vector())
        .norm();

        assert_abs_diff_eq!(
            transformed_distance,
            original_distance * scaling,
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity3_preserves_angles() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_45_x(), 2.0);
        let origin = Point3::new(0.0, 0.0, 0.0);
        let point1 = Point3::new(1.0, 0.0, 0.0);
        let point2 = Point3::new(0.0, 1.0, 0.0);

        let vec1 = point1.as_vector() - origin.as_vector();
        let vec2 = point2.as_vector() - origin.as_vector();
        let original_angle = vec1.dot(&vec2) / (vec1.norm() * vec2.norm());

        let t_origin = sim.transform_point(&origin);
        let t1 = sim.transform_point(&point1);
        let t2 = sim.transform_point(&point2);

        let t_vec1 = t1.as_vector() - t_origin.as_vector();
        let t_vec2 = t2.as_vector() - t_origin.as_vector();
        let transformed_angle = t_vec1.dot(&t_vec2) / (t_vec1.norm() * t_vec2.norm());

        assert_abs_diff_eq!(transformed_angle, original_angle, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_negative_scaling_works() {
        let sim = Similarity3::from_scaling(-2.0);
        let transformed = sim.transform_point(&Point3::new(1.0, 2.0, 3.0));

        assert_abs_diff_eq!(
            transformed,
            Point3::new(-2.0, -4.0, -6.0),
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity3_composing_many_transformations_accumulates_correctly() {
        let mut sim = Similarity3::identity();
        let small_translation = Vector3::new(0.001, 0.001, 0.001);
        let small_rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 0.01);
        let small_scaling = 1.01;

        for _ in 0..100 {
            sim = sim
                .translated(&small_translation)
                .rotated(&small_rotation)
                .scaled(small_scaling);
        }

        assert!(sim.translation().norm() < 1.0);
        assert!(sim.rotation().angle() < 2.0 * PI);
        assert!(sim.scaling() > 1.0 && sim.scaling() < 10.0);
    }

    // === Similarity3P Tests (packed) ===

    #[test]
    fn converting_similarity3p_to_aligned_and_back_preserves_data() {
        let rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 2.0).pack();
        let sim3 = Similarity3P::from_parts(Vector3P::new(7.0, 8.0, 9.0), rotation, 3.0);
        let sim3a = sim3.unpack();

        assert_abs_diff_eq!(
            *sim3a.translation(),
            Vector3::new(7.0, 8.0, 9.0),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim3a.scaling(), 3.0, epsilon = EPSILON);
    }

    #[test]
    fn converting_similarity3a_to_similarity3_preserves_components() {
        let sim3a = Similarity3::from_parts(
            Vector3::new(10.0, 11.0, 12.0),
            UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 6.0),
            0.5,
        );
        let sim3 = sim3a.pack();

        assert_abs_diff_eq!(
            *sim3.translation(),
            Vector3P::new(10.0, 11.0, 12.0),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim3.scaling(), 0.5, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_default_equals_identity() {
        let default = Similarity3P::default();
        let identity = Similarity3P::identity();

        assert_abs_diff_eq!(
            *default.translation(),
            *identity.translation(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*default.rotation(), *identity.rotation(), epsilon = EPSILON);
        assert_abs_diff_eq!(default.scaling(), identity.scaling(), epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_translation_has_identity_rotation_unit_scaling() {
        let translation = Vector3P::new(5.0, -3.0, 2.0);
        let sim = Similarity3P::from_translation(translation);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternionP::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_rotation_has_zero_translation_unit_scaling() {
        let rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), PI / 4.0).pack();
        let sim = Similarity3P::from_rotation(rotation);

        assert_abs_diff_eq!(*sim.translation(), Vector3P::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_scaling_has_zero_translation_identity_rotation() {
        let scaling = 2.5;
        let sim = Similarity3P::from_scaling(scaling);

        assert_abs_diff_eq!(*sim.translation(), Vector3P::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternionP::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_to_matrix_with_scaling_produces_scaled_identity() {
        let scaling = 2.0;
        let sim = Similarity3::from_scaling(scaling);
        let matrix = sim.to_matrix();

        let expected = Matrix4P::from_columns(
            Vector4P::new(2.0, 0.0, 0.0, 0.0),
            Vector4P::new(0.0, 2.0, 0.0, 0.0),
            Vector4P::new(0.0, 0.0, 2.0, 0.0),
            Vector4P::new(0.0, 0.0, 0.0, 1.0),
        )
        .unpack();

        assert_abs_diff_eq!(matrix, expected, epsilon = EPSILON);
    }
}
