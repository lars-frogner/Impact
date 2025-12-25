//! Similarity transforms.

use crate::{
    matrix::Matrix4A,
    point::Point3A,
    quaternion::{UnitQuaternion, UnitQuaternionA},
    transform::{Isometry3, Isometry3A},
    vector::{Vector3, Vector3A},
};
use bytemuck::{Pod, Zeroable};

/// A transform consisting of a uniform scaling and a rotation followed by a
/// translation.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Similarity3A`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct Similarity3 {
    rotation: UnitQuaternion,
    translation: Vector3,
    scaling: f32,
}

/// A transform consisting of a uniform scaling and a rotation followed by a
/// translation.
///
/// The rotation quaternion and translation vector are stored in 128-bit SIMD
/// registers for efficient computation. That leads to an extra 16 bytes in size
/// (4 due to the padded vector and 12 due to padding after the scale factor)
/// and 16-byte alignment. For cache-friendly storage, prefer [`Similarity3`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct Similarity3A {
    rotation: UnitQuaternionA,
    translation: Vector3A,
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

    /// Converts the transform to the 16-byte aligned SIMD-friendly
    /// [`Similarity3A`].
    #[inline]
    pub fn aligned(&self) -> Similarity3A {
        Similarity3A::from_parts(
            self.translation().aligned(),
            self.rotation().aligned(),
            self.scaling(),
        )
    }
}

impl Default for Similarity3 {
    fn default() -> Self {
        Self::identity()
    }
}

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

impl Similarity3A {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_parts(Vector3A::zeros(), UnitQuaternionA::identity(), 1.0)
    }

    /// Creates the similarity transform consisting of the given uniform
    /// scaling, rotation and translation.
    #[inline]
    pub const fn from_parts(
        translation: Vector3A,
        rotation: UnitQuaternionA,
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
    pub const fn from_isometry(isometry: Isometry3A) -> Self {
        Self::from_parts(*isometry.translation(), *isometry.rotation(), 1.0)
    }

    /// Creates the similarity transform consisting of the given translation and
    /// no rotation or scaling.
    #[inline]
    pub const fn from_translation(translation: Vector3A) -> Self {
        Self::from_parts(translation, UnitQuaternionA::identity(), 1.0)
    }

    /// Creates the similarity transform consisting of the given rotation and
    /// no translation or scaling.
    #[inline]
    pub const fn from_rotation(rotation: UnitQuaternionA) -> Self {
        Self::from_parts(Vector3A::zeros(), rotation, 1.0)
    }

    /// Creates the similarity transform consisting of the given scaling and no
    /// translation or rotation.
    #[inline]
    pub const fn from_scaling(scaling: f32) -> Self {
        Self::from_parts(Vector3A::zeros(), UnitQuaternionA::identity(), scaling)
    }

    /// Creates the similarity transform corresponding to applying the given
    /// translation before the scaling.
    #[inline]
    pub fn from_scaled_translation(translation: Vector3A, scaling: f32) -> Self {
        Self::from_parts(translation * scaling, UnitQuaternionA::identity(), scaling)
    }

    /// Creates the similarity transform corresponding to applying the given
    /// rotation before the scaling.
    #[inline]
    pub fn from_scaled_rotation(rotation: UnitQuaternionA, scaling: f32) -> Self {
        Self::from_rotation(rotation).scaled(scaling)
    }

    /// The translational part of the transform.
    #[inline]
    pub const fn translation(&self) -> &Vector3A {
        &self.translation
    }

    /// The rotational part of the transform.
    #[inline]
    pub const fn rotation(&self) -> &UnitQuaternionA {
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
    pub const fn to_isometry(&self) -> Isometry3A {
        Isometry3A::from_parts(self.translation, self.rotation)
    }

    /// Returns the transform where the given translation is applied after this
    /// transform.
    #[inline]
    pub fn translated(&self, translation: &Vector3A) -> Self {
        Self::from_parts(self.translation + translation, self.rotation, self.scaling)
    }

    /// Returns the transform where the given rotation is applied after this
    /// transform.
    #[inline]
    pub fn rotated(&self, rotation: &UnitQuaternionA) -> Self {
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
    pub fn applied_to_translation(&self, translation: &Vector3A) -> Self {
        Self::from_parts(
            self.rotation.rotate_vector(&(self.scaling * translation)) + self.translation,
            self.rotation,
            self.scaling,
        )
    }

    /// Returns the transform where the given rotation is applied before this
    /// transform.
    #[inline]
    pub fn applied_to_rotation(&self, rotation: &UnitQuaternionA) -> Self {
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
    pub fn to_matrix(&self) -> Matrix4A {
        let mut m = self.rotation.to_homogeneous_matrix();
        m.scale_transform(self.scaling);
        m.translate_transform(&self.translation);
        m
    }

    /// Applies the transform to the given point.
    #[inline]
    pub fn transform_point(&self, point: &Point3A) -> Point3A {
        self.rotation.rotate_point(&(self.scaling * point)) + self.translation
    }

    /// Applies the transform to the given vector. The translation part of the
    /// transform is not applied to vectors.
    #[inline]
    pub fn transform_vector(&self, vector: &Vector3A) -> Vector3A {
        self.rotation.rotate_vector(&(self.scaling * vector))
    }

    /// Applies the inverse of this transform to the given point. For a single
    /// transformation, this is more efficient than explicitly inverting the
    /// transform and then applying it.
    #[inline]
    pub fn inverse_transform_point(&self, point: &Point3A) -> Point3A {
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
    pub fn inverse_transform_vector(&self, vector: &Vector3A) -> Vector3A {
        self.rotation.inverse().rotate_vector(vector) / self.scaling
    }

    /// Converts the transform to the 4-byte aligned cache-friendly
    /// [`Similarity3`].
    #[inline]
    pub fn unaligned(&self) -> Similarity3 {
        Similarity3::from_parts(
            self.translation().unaligned(),
            self.rotation().unaligned(),
            self.scaling(),
        )
    }
}

impl Default for Similarity3A {
    fn default() -> Self {
        Self::identity()
    }
}

impl_binop!(Mul, mul, Similarity3A, Isometry3A, Similarity3A, |a, b| {
    Similarity3A::from_parts(
        a.rotation.rotate_vector(&(a.scaling * b.translation())) + a.translation,
        a.rotation * b.rotation(),
        a.scaling,
    )
});

impl_binop!(Mul, mul, Isometry3A, Similarity3A, Similarity3A, |a, b| {
    Similarity3A::from_parts(
        a.rotation().rotate_vector(&b.translation) + a.translation(),
        a.rotation() * b.rotation,
        b.scaling,
    )
});

impl_binop!(
    Mul,
    mul,
    Similarity3A,
    Similarity3A,
    Similarity3A,
    |a, b| {
        Similarity3A::from_parts(
            a.rotation.rotate_vector(&(a.scaling * b.translation)) + a.translation,
            a.rotation * b.rotation,
            a.scaling * b.scaling,
        )
    }
);

impl_abs_diff_eq!(Similarity3A, |a, b, epsilon| {
    a.rotation.abs_diff_eq(&b.rotation, epsilon)
        && a.translation.abs_diff_eq(&b.translation, epsilon)
        && a.scaling.abs_diff_eq(&b.scaling, epsilon)
});

impl_relative_eq!(Similarity3A, |a, b, epsilon, max_relative| {
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
        matrix::Matrix4,
        vector::{UnitVector3, Vector4},
    };
    use approx::assert_abs_diff_eq;
    use std::f32::consts::PI;

    // Test constants
    const EPSILON: f32 = 1e-6;
    const TRANSLATION_1: Vector3A = Vector3A::new(1.0, 2.0, 3.0);
    const TRANSLATION_2: Vector3A = Vector3A::new(4.0, 5.0, 6.0);

    fn rotation_90_z() -> UnitQuaternionA {
        UnitQuaternionA::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0)
    }

    fn rotation_45_x() -> UnitQuaternionA {
        UnitQuaternionA::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0)
    }

    // Identity tests
    #[test]
    fn creating_identity_similarity_gives_unit_scaling() {
        let sim = Similarity3A::identity();

        assert_abs_diff_eq!(*sim.translation(), Vector3A::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternionA::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn identity_similarity_equals_default() {
        let identity = Similarity3A::identity();
        let default = Similarity3A::default();

        assert_abs_diff_eq!(identity, default, epsilon = EPSILON);
    }

    // Construction tests
    #[test]
    fn creating_similarity_from_parts_stores_all_components() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.5;
        let sim = Similarity3A::from_parts(translation, rotation, scaling);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_isometry_has_unit_scaling() {
        let isometry = Isometry3A::from_parts(TRANSLATION_1, rotation_90_z());
        let sim = Similarity3A::from_isometry(isometry);

        assert_abs_diff_eq!(
            *sim.translation(),
            isometry.translation(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*sim.rotation(), *isometry.rotation(), epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_translation_has_identity_rotation_unit_scaling() {
        let translation = TRANSLATION_1;
        let sim = Similarity3A::from_translation(translation);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternionA::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_rotation_has_zero_translation_unit_scaling() {
        let rotation = rotation_90_z();
        let sim = Similarity3A::from_rotation(rotation);

        assert_abs_diff_eq!(*sim.translation(), Vector3A::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_scaling_has_zero_translation_identity_rotation() {
        let scaling = 2.5;
        let sim = Similarity3A::from_scaling(scaling);

        assert_abs_diff_eq!(*sim.translation(), Vector3A::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternionA::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_scaled_translation_scales_translation() {
        let translation = Vector3A::new(1.0, 2.0, 3.0);
        let scaling = 2.0;
        let sim = Similarity3A::from_scaled_translation(translation, scaling);

        let expected_translation = translation * scaling;
        assert_abs_diff_eq!(*sim.translation(), expected_translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternionA::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_scaled_rotation_works() {
        let rotation = rotation_90_z();
        let scaling = 3.0;
        let sim = Similarity3A::from_scaled_rotation(rotation, scaling);

        assert_abs_diff_eq!(*sim.translation(), Vector3A::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    // Transformation composition tests
    #[test]
    fn translating_similarity_adds_translation() {
        let sim = Similarity3A::from_translation(TRANSLATION_1);
        let additional_translation = TRANSLATION_2;
        let translated = sim.translated(&additional_translation);

        let expected_translation = TRANSLATION_1 + additional_translation;
        assert_abs_diff_eq!(
            *translated.translation(),
            expected_translation,
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*translated.rotation(), *sim.rotation(), epsilon = EPSILON);
        assert_abs_diff_eq!(translated.scaling(), sim.scaling(), epsilon = EPSILON);
    }

    #[test]
    fn rotating_similarity_composes_rotations() {
        let rotation1 = rotation_90_z();
        let rotation2 = rotation_45_x();
        let sim = Similarity3A::from_rotation(rotation1);
        let rotated = sim.rotated(&rotation2);

        let expected_rotation = rotation2 * rotation1;
        assert_abs_diff_eq!(*rotated.rotation(), expected_rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *rotated.translation(),
            *sim.translation(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(rotated.scaling(), sim.scaling(), epsilon = EPSILON);
    }

    #[test]
    fn scaling_similarity_multiplies_scaling() {
        let initial_scaling = 2.0;
        let additional_scaling = 3.0;
        let sim = Similarity3A::from_scaling(initial_scaling);
        let scaled = sim.scaled(additional_scaling);

        let expected_scaling = initial_scaling * additional_scaling;
        assert_abs_diff_eq!(scaled.scaling(), expected_scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(*scaled.translation(), *sim.translation(), epsilon = EPSILON);
        assert_abs_diff_eq!(*scaled.rotation(), *sim.rotation(), epsilon = EPSILON);
    }

    #[test]
    fn applying_to_translation_transforms_and_adds() {
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_90_z(), 2.0);
        let additional_translation = TRANSLATION_2;
        let result = sim.applied_to_translation(&additional_translation);

        // Should scale and rotate the additional translation, then add to existing
        let transformed_translation = sim.transform_vector(&additional_translation);
        let expected_translation = TRANSLATION_1 + transformed_translation;
        assert_abs_diff_eq!(
            *result.translation(),
            expected_translation,
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*result.rotation(), *sim.rotation(), epsilon = EPSILON);
        assert_abs_diff_eq!(result.scaling(), sim.scaling(), epsilon = EPSILON);
    }

    #[test]
    fn applying_to_rotation_composes_rotations_in_order() {
        let rotation1 = rotation_90_z();
        let rotation2 = rotation_45_x();
        let sim = Similarity3A::from_rotation(rotation1);
        let result = sim.applied_to_rotation(&rotation2);

        let expected_rotation = rotation1 * rotation2;
        assert_abs_diff_eq!(*result.rotation(), expected_rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(*result.translation(), *sim.translation(), epsilon = EPSILON);
        assert_abs_diff_eq!(result.scaling(), sim.scaling(), epsilon = EPSILON);
    }

    #[test]
    fn applying_to_scaling_prepends_scaling() {
        let initial_scaling = 2.0;
        let additional_scaling = 3.0;
        let sim = Similarity3A::from_scaling(initial_scaling);
        let result = sim.applied_to_scaling(additional_scaling);

        // Prepend scaling means additional_scaling is applied first
        let expected_scaling = initial_scaling * additional_scaling;
        assert_abs_diff_eq!(result.scaling(), expected_scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(*result.translation(), *sim.translation(), epsilon = EPSILON);
        assert_abs_diff_eq!(*result.rotation(), *sim.rotation(), epsilon = EPSILON);
    }

    // Inversion tests
    #[test]
    fn inverting_identity_gives_identity() {
        let identity = Similarity3A::identity();
        let inverted = identity.inverted();

        assert_abs_diff_eq!(inverted, identity, epsilon = EPSILON);
    }

    #[test]
    fn inverting_translation_gives_negative_translation() {
        let translation = TRANSLATION_1;
        let sim = Similarity3A::from_translation(translation);
        let inverted = sim.inverted();

        assert_abs_diff_eq!(*inverted.translation(), -translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *inverted.rotation(),
            UnitQuaternionA::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(inverted.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn inverting_rotation_gives_inverse_rotation() {
        let rotation = rotation_90_z();
        let sim = Similarity3A::from_rotation(rotation);
        let inverted = sim.inverted();

        assert_abs_diff_eq!(
            *inverted.translation(),
            Vector3A::zeros(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*inverted.rotation(), rotation.inverse(), epsilon = EPSILON);
        assert_abs_diff_eq!(inverted.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn inverting_scaling_gives_reciprocal_scaling() {
        let scaling = 2.5;
        let sim = Similarity3A::from_scaling(scaling);
        let inverted = sim.inverted();

        assert_abs_diff_eq!(
            *inverted.translation(),
            Vector3A::zeros(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            *inverted.rotation(),
            UnitQuaternionA::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(inverted.scaling(), 1.0 / scaling, epsilon = EPSILON);
    }

    #[test]
    fn similarity_times_inverse_gives_identity() {
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let inverted = sim.inverted();
        let result = sim * inverted;

        assert_abs_diff_eq!(result, Similarity3A::identity(), epsilon = EPSILON);
    }

    #[test]
    fn inverse_times_similarity_gives_identity() {
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let inverted = sim.inverted();
        let result = inverted * sim;

        assert_abs_diff_eq!(result, Similarity3A::identity(), epsilon = EPSILON);
    }

    // Matrix conversion tests
    #[test]
    fn to_matrix_produces_correct_homogeneous_matrix() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.0;
        let sim = Similarity3A::from_parts(translation, rotation, scaling);
        let matrix = sim.to_matrix();

        // Test by transforming a point
        let point = Point3A::new(1.0, 0.0, 0.0);
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
        assert_abs_diff_eq!(
            sim_transformed.z(),
            matrix_transformed.z(),
            epsilon = EPSILON
        );
    }

    // Component access tests
    #[test]
    fn isometry_component_access_works() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.0;
        let sim = Similarity3A::from_parts(translation, rotation, scaling);
        let isometry = sim.to_isometry();

        assert_abs_diff_eq!(*isometry.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*isometry.rotation(), rotation, epsilon = EPSILON);
    }

    #[test]
    fn component_getters_work_correctly() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.5;
        let sim = Similarity3A::from_parts(translation, rotation, scaling);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    // Point transformation tests
    #[test]
    fn transforming_point_with_identity_gives_same_point() {
        let point = Point3A::new(1.0, 2.0, 3.0);
        let identity = Similarity3A::identity();
        let transformed = identity.transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_translation_adds_translation() {
        let point = Point3A::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let sim = Similarity3A::from_translation(translation);
        let transformed = sim.transform_point(&point);

        let expected = Point3A::from(*point.as_vector() + translation);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_scaling_scales_coordinates() {
        let point = Point3A::new(1.0, 2.0, 3.0);
        let scaling = 2.5;
        let sim = Similarity3A::from_scaling(scaling);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), point.x() * scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), point.y() * scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), point.z() * scaling, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_rotation_rotates_point() {
        let point = Point3A::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let sim = Similarity3A::from_rotation(rotation);
        let transformed = sim.transform_point(&point);

        let expected_coords = rotation.rotate_vector(point.as_vector());
        let expected = Point3A::from(expected_coords);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_full_similarity_applies_all_components() {
        let point = Point3A::new(1.0, 0.0, 0.0);
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.0;
        let sim = Similarity3A::from_parts(translation, rotation, scaling);
        let transformed = sim.transform_point(&point);

        // Manual calculation: scale -> rotate -> translate
        let scaled = *point.as_vector() * scaling;
        let rotated = rotation.rotate_vector(&scaled);
        let expected = Point3A::from(rotated + translation);

        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    // Vector transformation tests
    #[test]
    fn transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3A::new(1.0, 2.0, 3.0);
        let identity = Similarity3A::identity();
        let transformed = identity.transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_translation_gives_same_vector() {
        let vector = Vector3A::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let sim = Similarity3A::from_translation(translation);
        let transformed = sim.transform_vector(&vector);

        // Vectors should not be affected by translation
        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_scaling_scales_vector() {
        let vector = Vector3A::new(1.0, 2.0, 3.0);
        let scaling = 2.5;
        let sim = Similarity3A::from_scaling(scaling);
        let transformed = sim.transform_vector(&vector);

        let expected = vector * scaling;
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_rotation_rotates_vector() {
        let vector = Vector3A::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let sim = Similarity3A::from_rotation(rotation);
        let transformed = sim.transform_vector(&vector);

        let expected = rotation.rotate_vector(&vector);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    // Inverse transformation tests
    #[test]
    fn inverse_transforming_point_with_identity_gives_same_point() {
        let point = Point3A::new(1.0, 2.0, 3.0);
        let identity = Similarity3A::identity();
        let transformed = identity.inverse_transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_point() {
        let point = Point3A::new(1.0, 2.0, 3.0);
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let transformed = sim.transform_point(&point);
        let back = sim.inverse_transform_point(&transformed);

        assert_abs_diff_eq!(back, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3A::new(1.0, 2.0, 3.0);
        let identity = Similarity3A::identity();
        let transformed = identity.inverse_transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_vector() {
        let vector = Vector3A::new(1.0, 2.0, 3.0);
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let transformed = sim.transform_vector(&vector);
        let back = sim.inverse_transform_vector(&transformed);

        assert_abs_diff_eq!(back, vector, epsilon = EPSILON);
    }

    // Multiplication tests
    #[test]
    fn multiplying_similarity_by_identity_similarity_gives_same_similarity() {
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_90_z(), 2.0);
        let identity = Similarity3A::identity();

        let result1 = &sim * &identity;
        let result2 = &identity * &sim;

        assert_abs_diff_eq!(result1, sim, epsilon = EPSILON);
        assert_abs_diff_eq!(result2, sim, epsilon = EPSILON);
    }

    #[test]
    fn multiplying_similarity_by_isometry_works() {
        let sim = Similarity3A::from_scaling(2.0);
        let iso = Isometry3A::from_translation(TRANSLATION_1);

        let result1 = &sim * &iso; // Similarity3A * Isometry3A -> Similarity3A
        let result2 = &iso * &sim; // Isometry3A * Similarity3A -> Similarity3A

        // Test by transforming a point
        let point = Point3A::new(1.0, 0.0, 0.0);

        // For sim * iso: first apply isometry, then similarity
        let expected1 = sim.transform_point(&iso.transform_point(&point));
        let actual1 = result1.transform_point(&point);
        assert_abs_diff_eq!(actual1, expected1, epsilon = EPSILON);

        // For iso * sim: first apply similarity, then isometry
        let expected2 = iso.transform_point(&sim.transform_point(&point));
        let actual2 = result2.transform_point(&point);
        assert_abs_diff_eq!(actual2, expected2, epsilon = EPSILON);
    }

    #[test]
    fn multiplying_similarities_composes_correctly() {
        let sim1 = Similarity3A::from_scaling(2.0);
        let sim2 = Similarity3A::from_translation(TRANSLATION_1);
        let composed = &sim2 * &sim1;

        let point = Point3A::new(1.0, 0.0, 0.0);

        // Manual composition: first sim1, then sim2
        let step1 = sim1.transform_point(&point);
        let step2 = sim2.transform_point(&step1);
        let direct = composed.transform_point(&point);

        assert_abs_diff_eq!(direct, step2, epsilon = EPSILON);
    }

    #[test]
    fn multiplication_is_associative() {
        let sim1 = Similarity3A::from_translation(TRANSLATION_1);
        let sim2 = Similarity3A::from_rotation(rotation_90_z());
        let sim3 = Similarity3A::from_scaling(2.0);

        let result1 = (&sim1 * &sim2) * &sim3;
        let result2 = &sim1 * (&sim2 * &sim3);

        assert_abs_diff_eq!(result1, result2, epsilon = EPSILON);
    }

    // Property tests
    #[test]
    fn similarity_preserves_ratios_of_distances() {
        let scaling = 2.5;
        let sim = Similarity3A::from_scaling(scaling);

        let point1 = Point3A::new(0.0, 0.0, 0.0);
        let point2 = Point3A::new(1.0, 0.0, 0.0);
        let point3 = Point3A::new(2.0, 0.0, 0.0);

        let original_dist12 = (point2.as_vector() - point1.as_vector()).norm();
        let original_dist13 = (point3.as_vector() - point1.as_vector()).norm();
        let original_ratio = original_dist13 / original_dist12;

        let t1 = sim.transform_point(&point1);
        let t2 = sim.transform_point(&point2);
        let t3 = sim.transform_point(&point3);

        let transformed_dist12 = (t2.as_vector() - t1.as_vector()).norm();
        let transformed_dist13 = (t3.as_vector() - t1.as_vector()).norm();
        let transformed_ratio = transformed_dist13 / transformed_dist12;

        assert_abs_diff_eq!(transformed_ratio, original_ratio, epsilon = EPSILON);
    }

    #[test]
    fn similarity_scales_distances_uniformly() {
        let scaling = 3.0;
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_45_x(), scaling);

        let point1 = Point3A::new(1.0, 2.0, 3.0);
        let point2 = Point3A::new(4.0, 5.0, 6.0);

        let original_distance = (point2.as_vector() - point1.as_vector()).norm();

        let t1 = sim.transform_point(&point1);
        let t2 = sim.transform_point(&point2);
        let transformed_distance = (t2.as_vector() - t1.as_vector()).norm();

        assert_abs_diff_eq!(
            transformed_distance,
            original_distance * scaling,
            epsilon = EPSILON
        );
    }

    #[test]
    fn similarity_preserves_angles() {
        let sim = Similarity3A::from_parts(TRANSLATION_1, rotation_45_x(), 2.0);
        let origin = Point3A::new(0.0, 0.0, 0.0);
        let point1 = Point3A::new(1.0, 0.0, 0.0);
        let point2 = Point3A::new(0.0, 1.0, 0.0);

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

    // Approximate equality tests
    #[test]
    fn abs_diff_eq_works_with_small_differences() {
        let sim1 = Similarity3A::from_translation(Vector3A::new(1.0, 2.0, 3.0));
        let sim2 = Similarity3A::from_translation(Vector3A::new(1.0 + 1e-7, 2.0, 3.0));

        assert_abs_diff_eq!(sim1, sim2, epsilon = 1e-6);
    }

    #[test]
    fn relative_eq_works_with_proportional_differences() {
        let sim1 = Similarity3A::from_scaling(2.0);
        let sim2 = Similarity3A::from_scaling(2.00001);

        use approx::assert_relative_eq;
        assert_relative_eq!(sim1, sim2, epsilon = 1e-6, max_relative = 1e-4);
    }

    // Edge case tests
    #[test]
    fn very_small_scaling_works() {
        let small_scaling = 1e-6;
        let sim = Similarity3A::from_scaling(small_scaling);

        assert_abs_diff_eq!(sim.scaling(), small_scaling, epsilon = 1e-9);

        let point = Point3A::new(1000.0, 1000.0, 1000.0);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), point.x() * small_scaling, epsilon = 1e-6);
        assert_abs_diff_eq!(transformed.y(), point.y() * small_scaling, epsilon = 1e-6);
        assert_abs_diff_eq!(transformed.z(), point.z() * small_scaling, epsilon = 1e-6);
    }

    #[test]
    fn large_scaling_works() {
        let large_scaling = 1e6;
        let sim = Similarity3A::from_scaling(large_scaling);

        assert_abs_diff_eq!(sim.scaling(), large_scaling, epsilon = 1e-3);

        let point = Point3A::new(1e-3, 1e-3, 1e-3);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), point.x() * large_scaling, epsilon = 1e-3);
        assert_abs_diff_eq!(transformed.y(), point.y() * large_scaling, epsilon = 1e-3);
        assert_abs_diff_eq!(transformed.z(), point.z() * large_scaling, epsilon = 1e-3);
    }

    #[test]
    fn negative_scaling_works() {
        let negative_scaling = -2.0;
        let sim = Similarity3A::from_scaling(negative_scaling);

        assert_abs_diff_eq!(sim.scaling(), negative_scaling, epsilon = EPSILON);

        let point = Point3A::new(1.0, 2.0, 3.0);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(
            transformed.x(),
            point.x() * negative_scaling,
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            transformed.y(),
            point.y() * negative_scaling,
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(
            transformed.z(),
            point.z() * negative_scaling,
            epsilon = EPSILON
        );
    }

    #[test]
    fn composing_many_small_transformations_works() {
        let mut sim = Similarity3A::identity();
        let small_translation = Vector3A::new(0.001, 0.001, 0.001);
        let small_rotation = UnitQuaternionA::from_axis_angle(&UnitVector3::unit_z(), 0.01);
        let small_scaling = 1.01;

        for _ in 0..100 {
            sim = sim.translated(&small_translation);
            sim = sim.rotated(&small_rotation);
            sim = sim.scaled(small_scaling);
        }

        // Should accumulate to reasonable values
        assert!(sim.translation().norm() < 1.0);
        assert!(sim.rotation().angle() < 2.0 * PI);
        assert!(sim.scaling() > 1.0);
        assert!(sim.scaling() < 10.0);
    }

    // Similarity3 specific tests
    #[test]
    fn similarity3_identity_has_unit_components() {
        let sim = Similarity3::identity();

        assert_abs_diff_eq!(*sim.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_parts_stores_components_correctly() {
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let rotation =
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_y(), PI / 3.0).unaligned();
        let scaling = 2.0;
        let sim = Similarity3::from_parts(translation, rotation, scaling);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_isometry_has_unit_scaling() {
        let isometry = Isometry3::from_parts(
            Vector3::new(1.0, 2.0, 3.0),
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_x(), PI / 6.0).unaligned(),
        );
        let sim = Similarity3::from_isometry(isometry);

        assert_abs_diff_eq!(
            *sim.translation(),
            *isometry.translation(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(*sim.rotation(), *isometry.rotation(), epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_aligned_returns_similarity3a() {
        let sim3 = Similarity3::from_parts(
            Vector3::new(1.0, 2.0, 3.0),
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_z(), PI / 4.0).unaligned(),
            1.5,
        );
        let sim3a = sim3.aligned();

        assert_abs_diff_eq!(
            *sim3a.translation(),
            Vector3A::new(1.0, 2.0, 3.0),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim3a.scaling(), 1.5, epsilon = EPSILON);
    }

    #[test]
    fn similarity3a_unaligned_returns_similarity3() {
        let sim3a = Similarity3A::from_parts(
            Vector3A::new(4.0, 5.0, 6.0),
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_y(), PI / 3.0),
            0.8,
        );
        let sim3 = sim3a.unaligned();

        assert_abs_diff_eq!(
            *sim3.translation(),
            Vector3::new(4.0, 5.0, 6.0),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim3.scaling(), 0.8, epsilon = EPSILON);
    }

    // Conversion trait tests
    #[test]
    fn converting_similarity3_to_similarity3a_preserves_components() {
        let rotation =
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_x(), PI / 2.0).unaligned();
        let sim3 = Similarity3::from_parts(Vector3::new(7.0, 8.0, 9.0), rotation, 3.0);
        let sim3a = sim3.aligned();

        assert_abs_diff_eq!(
            *sim3a.translation(),
            Vector3A::new(7.0, 8.0, 9.0),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim3a.scaling(), 3.0, epsilon = EPSILON);
    }

    #[test]
    fn converting_similarity3a_to_similarity3_preserves_components() {
        let sim3a = Similarity3A::from_parts(
            Vector3A::new(10.0, 11.0, 12.0),
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_z(), PI / 6.0),
            0.5,
        );
        let sim3 = sim3a.unaligned();

        assert_abs_diff_eq!(
            *sim3.translation(),
            Vector3::new(10.0, 11.0, 12.0),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim3.scaling(), 0.5, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_default_equals_identity() {
        let default = Similarity3::default();
        let identity = Similarity3::identity();

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
        let translation = Vector3::new(5.0, -3.0, 2.0);
        let sim = Similarity3::from_translation(translation);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_rotation_has_zero_translation_unit_scaling() {
        let rotation =
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_y(), PI / 4.0).unaligned();
        let sim = Similarity3::from_rotation(rotation);

        assert_abs_diff_eq!(*sim.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_from_scaling_has_zero_translation_identity_rotation() {
        let scaling = 2.5;
        let sim = Similarity3::from_scaling(scaling);

        assert_abs_diff_eq!(*sim.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn similarity3_to_isometry_drops_scaling() {
        let rotation =
            UnitQuaternionA::from_axis_angle(&UnitVector3::unit_z(), PI / 4.0).unaligned();
        let sim = Similarity3::from_parts(Vector3::new(1.0, 2.0, 3.0), rotation, 3.0);
        let iso = sim.to_isometry();

        assert_abs_diff_eq!(*iso.translation(), *sim.translation(), epsilon = EPSILON);
        assert_abs_diff_eq!(*iso.rotation(), *sim.rotation(), epsilon = EPSILON);
    }

    // Additional matrix tests
    #[test]
    fn to_matrix_with_identity_gives_identity_matrix() {
        let sim = Similarity3A::identity();
        let matrix = sim.to_matrix();

        assert_abs_diff_eq!(matrix, Matrix4::identity().aligned(), epsilon = EPSILON);
    }

    #[test]
    fn to_matrix_with_scaling_only_gives_scaled_identity() {
        let scaling = 2.0;
        let sim = Similarity3A::from_scaling(scaling);
        let matrix = sim.to_matrix();

        let expected = Matrix4::from_columns(
            Vector4::new(2.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 2.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 2.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        )
        .aligned();

        assert_abs_diff_eq!(matrix, expected, epsilon = EPSILON);
    }

    #[test]
    fn to_matrix_with_translation_only_gives_translation_matrix() {
        let translation = Vector3A::new(3.0, 4.0, 5.0);
        let sim = Similarity3A::from_translation(translation);
        let matrix = sim.to_matrix();

        let expected = Matrix4::from_columns(
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(3.0, 4.0, 5.0, 1.0),
        )
        .aligned();

        assert_abs_diff_eq!(matrix, expected, epsilon = EPSILON);
    }

    // Edge case tests
    #[test]
    fn zero_scaling_works() {
        let sim = Similarity3A::from_scaling(0.0);
        let point = Point3A::new(1.0, 2.0, 3.0);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(transformed, Point3A::origin(), epsilon = EPSILON);
    }

    #[test]
    fn roundtrip_conversion_preserves_similarity() {
        let original = Similarity3A::from_parts(Vector3A::new(1.0, 2.0, 3.0), rotation_45_x(), 2.0);
        let converted = original.unaligned().aligned();

        assert_abs_diff_eq!(converted, original, epsilon = EPSILON);
    }
}
