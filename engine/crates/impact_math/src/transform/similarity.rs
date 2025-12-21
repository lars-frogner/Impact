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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::UnitVector3;
    use approx::assert_abs_diff_eq;
    use std::f32::consts::PI;

    // Test constants
    const EPSILON: f32 = 1e-6;
    const TRANSLATION_1: Vector3 = Vector3::new(1.0, 2.0, 3.0);
    const TRANSLATION_2: Vector3 = Vector3::new(4.0, 5.0, 6.0);

    fn rotation_90_z() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), PI / 2.0)
    }

    fn rotation_45_x() -> UnitQuaternion {
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), PI / 4.0)
    }

    // Identity tests
    #[test]
    fn creating_identity_similarity_gives_unit_scaling() {
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
    fn identity_similarity_equals_default() {
        let identity = Similarity3::identity();
        let default = Similarity3::default();

        assert_abs_diff_eq!(identity, default, epsilon = EPSILON);
    }

    // Construction tests
    #[test]
    fn creating_similarity_from_parts_stores_all_components() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.5;
        let sim = Similarity3::from_parts(translation, rotation, scaling);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_isometry_has_unit_scaling() {
        let isometry = Isometry3::from_parts(TRANSLATION_1, rotation_90_z());
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
    fn creating_similarity_from_translation_has_identity_rotation_unit_scaling() {
        let translation = TRANSLATION_1;
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
    fn creating_similarity_from_rotation_has_zero_translation_unit_scaling() {
        let rotation = rotation_90_z();
        let sim = Similarity3::from_rotation(rotation);

        assert_abs_diff_eq!(*sim.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_scaling_has_zero_translation_identity_rotation() {
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
    fn creating_similarity_from_scaled_translation_scales_translation() {
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let scaling = 2.0;
        let sim = Similarity3::from_scaled_translation(translation, scaling);

        let expected_translation = translation * scaling;
        assert_abs_diff_eq!(*sim.translation(), expected_translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *sim.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    #[test]
    fn creating_similarity_from_scaled_rotation_works() {
        let rotation = rotation_90_z();
        let scaling = 3.0;
        let sim = Similarity3::from_scaled_rotation(rotation, scaling);

        assert_abs_diff_eq!(*sim.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    // Transformation composition tests
    #[test]
    fn translating_similarity_adds_translation() {
        let sim = Similarity3::from_translation(TRANSLATION_1);
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
        let sim = Similarity3::from_rotation(rotation1);
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
        let sim = Similarity3::from_scaling(initial_scaling);
        let scaled = sim.scaled(additional_scaling);

        let expected_scaling = initial_scaling * additional_scaling;
        assert_abs_diff_eq!(scaled.scaling(), expected_scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(*scaled.translation(), *sim.translation(), epsilon = EPSILON);
        assert_abs_diff_eq!(*scaled.rotation(), *sim.rotation(), epsilon = EPSILON);
    }

    #[test]
    fn applying_to_translation_transforms_and_adds() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.0);
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
        let sim = Similarity3::from_rotation(rotation1);
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
        let sim = Similarity3::from_scaling(initial_scaling);
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
        let identity = Similarity3::identity();
        let inverted = identity.inverted();

        assert_abs_diff_eq!(inverted, identity, epsilon = EPSILON);
    }

    #[test]
    fn inverting_translation_gives_negative_translation() {
        let translation = TRANSLATION_1;
        let sim = Similarity3::from_translation(translation);
        let inverted = sim.inverted();

        assert_abs_diff_eq!(*inverted.translation(), -translation, epsilon = EPSILON);
        assert_abs_diff_eq!(
            *inverted.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(inverted.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn inverting_rotation_gives_inverse_rotation() {
        let rotation = rotation_90_z();
        let sim = Similarity3::from_rotation(rotation);
        let inverted = sim.inverted();

        assert_abs_diff_eq!(*inverted.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(*inverted.rotation(), rotation.inverse(), epsilon = EPSILON);
        assert_abs_diff_eq!(inverted.scaling(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn inverting_scaling_gives_reciprocal_scaling() {
        let scaling = 2.5;
        let sim = Similarity3::from_scaling(scaling);
        let inverted = sim.inverted();

        assert_abs_diff_eq!(*inverted.translation(), Vector3::zeros(), epsilon = EPSILON);
        assert_abs_diff_eq!(
            *inverted.rotation(),
            UnitQuaternion::identity(),
            epsilon = EPSILON
        );
        assert_abs_diff_eq!(inverted.scaling(), 1.0 / scaling, epsilon = EPSILON);
    }

    #[test]
    fn similarity_times_inverse_gives_identity() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let inverted = sim.inverted();
        let result = sim * inverted;

        assert_abs_diff_eq!(result, Similarity3::identity(), epsilon = EPSILON);
    }

    #[test]
    fn inverse_times_similarity_gives_identity() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let inverted = sim.inverted();
        let result = inverted * sim;

        assert_abs_diff_eq!(result, Similarity3::identity(), epsilon = EPSILON);
    }

    // Matrix conversion tests
    #[test]
    fn to_matrix_produces_correct_homogeneous_matrix() {
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
        let sim = Similarity3::from_parts(translation, rotation, scaling);
        let isometry = sim.isometry();

        assert_abs_diff_eq!(*isometry.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*isometry.rotation(), rotation, epsilon = EPSILON);
    }

    #[test]
    fn component_getters_work_correctly() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.5;
        let sim = Similarity3::from_parts(translation, rotation, scaling);

        assert_abs_diff_eq!(*sim.translation(), translation, epsilon = EPSILON);
        assert_abs_diff_eq!(*sim.rotation(), rotation, epsilon = EPSILON);
        assert_abs_diff_eq!(sim.scaling(), scaling, epsilon = EPSILON);
    }

    // Point transformation tests
    #[test]
    fn transforming_point_with_identity_gives_same_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let identity = Similarity3::identity();
        let transformed = identity.transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_translation_adds_translation() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let sim = Similarity3::from_translation(translation);
        let transformed = sim.transform_point(&point);

        let expected = Point3::from(*point.as_vector() + translation);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_scaling_scales_coordinates() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let scaling = 2.5;
        let sim = Similarity3::from_scaling(scaling);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), point.x() * scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), point.y() * scaling, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), point.z() * scaling, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_rotation_rotates_point() {
        let point = Point3::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let sim = Similarity3::from_rotation(rotation);
        let transformed = sim.transform_point(&point);

        let expected_coords = rotation.transform_vector(point.as_vector());
        let expected = Point3::from(expected_coords);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_full_similarity_applies_all_components() {
        let point = Point3::new(1.0, 0.0, 0.0);
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scaling = 2.0;
        let sim = Similarity3::from_parts(translation, rotation, scaling);
        let transformed = sim.transform_point(&point);

        // Manual calculation: scale -> rotate -> translate
        let scaled = *point.as_vector() * scaling;
        let rotated = rotation.transform_vector(&scaled);
        let expected = Point3::from(rotated + translation);

        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    // Vector transformation tests
    #[test]
    fn transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let identity = Similarity3::identity();
        let transformed = identity.transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_translation_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let sim = Similarity3::from_translation(translation);
        let transformed = sim.transform_vector(&vector);

        // Vectors should not be affected by translation
        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_scaling_scales_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let scaling = 2.5;
        let sim = Similarity3::from_scaling(scaling);
        let transformed = sim.transform_vector(&vector);

        let expected = vector * scaling;
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_rotation_rotates_vector() {
        let vector = Vector3::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let sim = Similarity3::from_rotation(rotation);
        let transformed = sim.transform_vector(&vector);

        let expected = rotation.transform_vector(&vector);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    // Inverse transformation tests
    #[test]
    fn inverse_transforming_point_with_identity_gives_same_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let identity = Similarity3::identity();
        let transformed = identity.inverse_transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let transformed = sim.transform_point(&point);
        let back = sim.inverse_transform_point(&transformed);

        assert_abs_diff_eq!(back, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let identity = Similarity3::identity();
        let transformed = identity.inverse_transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.5);
        let transformed = sim.transform_vector(&vector);
        let back = sim.inverse_transform_vector(&transformed);

        assert_abs_diff_eq!(back, vector, epsilon = EPSILON);
    }

    // Multiplication tests
    #[test]
    fn multiplying_similarity_by_identity_similarity_gives_same_similarity() {
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_90_z(), 2.0);
        let identity = Similarity3::identity();

        let result1 = sim * identity;
        let result2 = identity * sim;

        assert_abs_diff_eq!(result1, sim, epsilon = EPSILON);
        assert_abs_diff_eq!(result2, sim, epsilon = EPSILON);
    }

    #[test]
    fn multiplying_similarity_by_isometry_works() {
        let sim = Similarity3::from_scaling(2.0);
        let iso = Isometry3::from_translation(TRANSLATION_1);

        let result1 = sim * iso; // Similarity3 * Isometry3 -> Similarity3
        let result2 = iso * sim; // Isometry3 * Similarity3 -> Similarity3

        // Test by transforming a point
        let point = Point3::new(1.0, 0.0, 0.0);

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
        let sim1 = Similarity3::from_scaling(2.0);
        let sim2 = Similarity3::from_translation(TRANSLATION_1);
        let composed = sim2 * sim1;

        let point = Point3::new(1.0, 0.0, 0.0);

        // Manual composition: first sim1, then sim2
        let step1 = sim1.transform_point(&point);
        let step2 = sim2.transform_point(&step1);
        let direct = composed.transform_point(&point);

        assert_abs_diff_eq!(direct, step2, epsilon = EPSILON);
    }

    #[test]
    fn multiplication_is_associative() {
        let sim1 = Similarity3::from_translation(TRANSLATION_1);
        let sim2 = Similarity3::from_rotation(rotation_90_z());
        let sim3 = Similarity3::from_scaling(2.0);

        let result1 = (sim1 * sim2) * sim3;
        let result2 = sim1 * (sim2 * sim3);

        assert_abs_diff_eq!(result1, result2, epsilon = EPSILON);
    }

    // Property tests
    #[test]
    fn similarity_preserves_ratios_of_distances() {
        let scaling = 2.5;
        let sim = Similarity3::from_scaling(scaling);

        let point1 = Point3::new(0.0, 0.0, 0.0);
        let point2 = Point3::new(1.0, 0.0, 0.0);
        let point3 = Point3::new(2.0, 0.0, 0.0);

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
        let sim = Similarity3::from_parts(TRANSLATION_1, rotation_45_x(), scaling);

        let point1 = Point3::new(1.0, 2.0, 3.0);
        let point2 = Point3::new(4.0, 5.0, 6.0);

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

    // Approximate equality tests
    #[test]
    fn abs_diff_eq_works_with_small_differences() {
        let sim1 = Similarity3::from_translation(Vector3::new(1.0, 2.0, 3.0));
        let sim2 = Similarity3::from_translation(Vector3::new(1.0 + 1e-7, 2.0, 3.0));

        assert_abs_diff_eq!(sim1, sim2, epsilon = 1e-6);
    }

    #[test]
    fn relative_eq_works_with_proportional_differences() {
        let sim1 = Similarity3::from_scaling(2.0);
        let sim2 = Similarity3::from_scaling(2.00001);

        use approx::assert_relative_eq;
        assert_relative_eq!(sim1, sim2, epsilon = 1e-6, max_relative = 1e-4);
    }

    // Edge case tests
    #[test]
    fn very_small_scaling_works() {
        let small_scaling = 1e-6;
        let sim = Similarity3::from_scaling(small_scaling);

        assert_abs_diff_eq!(sim.scaling(), small_scaling, epsilon = 1e-9);

        let point = Point3::new(1000.0, 1000.0, 1000.0);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), point.x() * small_scaling, epsilon = 1e-6);
        assert_abs_diff_eq!(transformed.y(), point.y() * small_scaling, epsilon = 1e-6);
        assert_abs_diff_eq!(transformed.z(), point.z() * small_scaling, epsilon = 1e-6);
    }

    #[test]
    fn large_scaling_works() {
        let large_scaling = 1e6;
        let sim = Similarity3::from_scaling(large_scaling);

        assert_abs_diff_eq!(sim.scaling(), large_scaling, epsilon = 1e-3);

        let point = Point3::new(1e-3, 1e-3, 1e-3);
        let transformed = sim.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), point.x() * large_scaling, epsilon = 1e-3);
        assert_abs_diff_eq!(transformed.y(), point.y() * large_scaling, epsilon = 1e-3);
        assert_abs_diff_eq!(transformed.z(), point.z() * large_scaling, epsilon = 1e-3);
    }

    #[test]
    fn negative_scaling_works() {
        let negative_scaling = -2.0;
        let sim = Similarity3::from_scaling(negative_scaling);

        assert_abs_diff_eq!(sim.scaling(), negative_scaling, epsilon = EPSILON);

        let point = Point3::new(1.0, 2.0, 3.0);
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
        let mut sim = Similarity3::identity();
        let small_translation = Vector3::new(0.001, 0.001, 0.001);
        let small_rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 0.01);
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
}
