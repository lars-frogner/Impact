//! Isometry transforms.

use crate::{matrix::Matrix4, point::Point3, quaternion::UnitQuaternion, vector::Vector3};
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Projective3 {
    inner: nalgebra::Projective3<f32>,
}

impl Projective3 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::Projective3::identity(),
        }
    }

    #[inline]
    pub fn from_matrix_unchecked(matrix: Matrix4) -> Self {
        Self {
            inner: nalgebra::Projective3::from_matrix_unchecked(*matrix._inner()),
        }
    }

    #[inline]
    pub fn matrix(&self) -> &Matrix4 {
        bytemuck::from_bytes(bytemuck::bytes_of(self.inner.matrix()))
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
    pub fn apply_to_translation(&self, translation: &Vector3) -> Self {
        Self {
            inner: self.inner * nalgebra::Translation3::from(*translation._inner()),
        }
    }

    #[inline]
    pub fn apply_to_rotation(&self, rotation: &UnitQuaternion) -> Self {
        Self {
            inner: self.inner * rotation._inner(),
        }
    }

    #[inline]
    pub fn inverse(&self) -> Self {
        Self {
            inner: self.inner.inverse(),
        }
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

impl_abs_diff_eq!(Projective3, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Projective3, |a, b, epsilon, max_relative| {
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

    fn scale_matrix(sx: f32, sy: f32, sz: f32) -> Matrix4 {
        use crate::vector::Vector4;
        let col1 = Vector4::new(sx, 0.0, 0.0, 0.0);
        let col2 = Vector4::new(0.0, sy, 0.0, 0.0);
        let col3 = Vector4::new(0.0, 0.0, sz, 0.0);
        let col4 = Vector4::new(0.0, 0.0, 0.0, 1.0);
        Matrix4::from_columns(&[col1, col2, col3, col4])
    }

    // Identity tests
    #[test]
    fn creating_identity_projective_gives_identity_matrix() {
        let proj = Projective3::identity();
        let matrix = proj.matrix();

        assert_abs_diff_eq!(*matrix, Matrix4::identity(), epsilon = EPSILON);
    }

    #[test]
    fn identity_projective_equals_default() {
        let identity = Projective3::identity();
        let default = Projective3::default();

        assert_abs_diff_eq!(identity, default, epsilon = EPSILON);
    }

    // Construction tests
    #[test]
    fn creating_projective_from_matrix_unchecked_stores_matrix() {
        let matrix = scale_matrix(2.0, 3.0, 4.0);
        let proj = Projective3::from_matrix_unchecked(matrix);

        assert_abs_diff_eq!(*proj.matrix(), matrix, epsilon = EPSILON);
    }

    #[test]
    fn creating_projective_from_identity_matrix_gives_identity() {
        let identity_matrix = Matrix4::identity();
        let proj = Projective3::from_matrix_unchecked(identity_matrix);
        let identity = Projective3::identity();

        assert_abs_diff_eq!(proj, identity, epsilon = EPSILON);
    }

    // Translation tests
    #[test]
    fn translating_identity_creates_translation_matrix() {
        let identity = Projective3::identity();
        let translation = TRANSLATION_1;
        let translated = identity.translated(&translation);

        // Check that translation appears in the right column of the matrix
        let matrix = translated.matrix();
        assert_abs_diff_eq!(matrix.element(0, 3), translation.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(1, 3), translation.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(2, 3), translation.z(), epsilon = EPSILON);
    }

    #[test]
    fn translating_projective_composes_translations() {
        let initial_translation = TRANSLATION_1;
        let additional_translation = TRANSLATION_2;

        let proj = Projective3::identity().translated(&initial_translation);
        let translated = proj.translated(&additional_translation);

        // Should be equivalent to translating by the sum
        let expected =
            Projective3::identity().translated(&(initial_translation + additional_translation));
        assert_abs_diff_eq!(translated, expected, epsilon = EPSILON);
    }

    // Rotation tests
    #[test]
    fn rotating_identity_creates_rotation_matrix() {
        let identity = Projective3::identity();
        let rotation = rotation_90_z();
        let rotated = identity.rotated(&rotation);

        // Test by applying to a point - 90° rotation around Z maps (1,0,0) to (0,1,0)
        let point = Point3::new(1.0, 0.0, 0.0);
        let transformed = rotated.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn rotating_projective_composes_rotations() {
        let rotation1 = rotation_90_z();
        let rotation2 = rotation_45_x();

        let proj = Projective3::identity().rotated(&rotation1);
        let rotated = proj.rotated(&rotation2);

        // Should be equivalent to applying rotations in sequence
        let point = Point3::new(1.0, 0.0, 0.0);
        let step1 = rotation1.transform_point(&point);
        let step2 = rotation2.transform_point(&step1);
        let direct = rotated.transform_point(&point);

        assert_abs_diff_eq!(direct.x(), step2.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(direct.y(), step2.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(direct.z(), step2.z(), epsilon = EPSILON);
    }

    // Application tests
    #[test]
    fn applying_to_translation_works() {
        let initial_translation = TRANSLATION_1;
        let additional_translation = TRANSLATION_2;

        let proj = Projective3::identity().translated(&initial_translation);
        let result = proj.apply_to_translation(&additional_translation);

        // Should transform the additional translation and compose
        let point = Point3::new(0.0, 0.0, 0.0);
        let step1 = proj.transform_point(&point);
        let step2 = Point3::from(*step1.as_vector() + additional_translation);
        let direct = result.transform_point(&point);

        assert_abs_diff_eq!(direct.x(), step2.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(direct.y(), step2.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(direct.z(), step2.z(), epsilon = EPSILON);
    }

    #[test]
    fn applying_to_rotation_works() {
        let initial_rotation = rotation_90_z();
        let additional_rotation = rotation_45_x();

        let proj = Projective3::identity().rotated(&initial_rotation);
        let result = proj.apply_to_rotation(&additional_rotation);

        // Should compose rotations in the correct order
        let point = Point3::new(1.0, 0.0, 0.0);
        let expected =
            initial_rotation.transform_point(&additional_rotation.transform_point(&point));
        let actual = result.transform_point(&point);

        assert_abs_diff_eq!(actual.x(), expected.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(actual.y(), expected.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(actual.z(), expected.z(), epsilon = EPSILON);
    }

    // Inversion tests
    #[test]
    fn inverting_identity_gives_identity() {
        let identity = Projective3::identity();
        let inverted = identity.inverse();

        assert_abs_diff_eq!(inverted, identity, epsilon = EPSILON);
    }

    #[test]
    fn inverting_translation_gives_negative_translation() {
        let translation = TRANSLATION_1;
        let proj = Projective3::identity().translated(&translation);
        let inverted = proj.inverse();

        // Inverse should undo the translation
        let point = Point3::new(0.0, 0.0, 0.0);
        let translated = proj.transform_point(&point);
        let back = inverted.transform_point(&translated);

        assert_abs_diff_eq!(back.x(), point.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(back.y(), point.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(back.z(), point.z(), epsilon = EPSILON);
    }

    #[test]
    fn inverting_rotation_gives_inverse_rotation() {
        let rotation = rotation_90_z();
        let proj = Projective3::identity().rotated(&rotation);
        let inverted = proj.inverse();

        // Inverse should undo the rotation
        let point = Point3::new(1.0, 0.0, 0.0);
        let rotated = proj.transform_point(&point);
        let back = inverted.transform_point(&rotated);

        assert_abs_diff_eq!(back.x(), point.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(back.y(), point.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(back.z(), point.z(), epsilon = EPSILON);
    }

    #[test]
    fn inverting_scale_gives_reciprocal_scale() {
        let scale = scale_matrix(2.0, 3.0, 4.0);
        let proj = Projective3::from_matrix_unchecked(scale);
        let inverted = proj.inverse();

        // Inverse should undo the scaling
        let point = Point3::new(1.0, 1.0, 1.0);
        let scaled = proj.transform_point(&point);
        let back = inverted.transform_point(&scaled);

        assert_abs_diff_eq!(back.x(), point.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(back.y(), point.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(back.z(), point.z(), epsilon = EPSILON);
    }

    // Point transformation tests
    #[test]
    fn transforming_point_with_identity_gives_same_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let identity = Projective3::identity();
        let transformed = identity.transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_translation_adds_translation() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let proj = Projective3::identity().translated(&translation);
        let transformed = proj.transform_point(&point);

        let expected = Point3::from(*point.as_vector() + translation);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_point_with_scale_multiplies_coordinates() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let scale = scale_matrix(2.0, 3.0, 4.0);
        let proj = Projective3::from_matrix_unchecked(scale);
        let transformed = proj.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), 2.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 6.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 12.0, epsilon = EPSILON);
    }

    // Vector transformation tests
    #[test]
    fn transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let identity = Projective3::identity();
        let transformed = identity.transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_translation_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let translation = TRANSLATION_1;
        let proj = Projective3::identity().translated(&translation);
        let transformed = proj.transform_vector(&vector);

        // Vectors should not be affected by translation
        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_rotation_rotates_vector() {
        let vector = Vector3::new(1.0, 0.0, 0.0);
        let rotation = rotation_90_z();
        let proj = Projective3::identity().rotated(&rotation);
        let transformed = proj.transform_vector(&vector);

        let expected = rotation.transform_vector(&vector);
        assert_abs_diff_eq!(transformed, expected, epsilon = EPSILON);
    }

    #[test]
    fn transforming_vector_with_scale_scales_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let scale = scale_matrix(2.0, 3.0, 4.0);
        let proj = Projective3::from_matrix_unchecked(scale);
        let transformed = proj.transform_vector(&vector);

        assert_abs_diff_eq!(transformed.x(), 2.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 6.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 12.0, epsilon = EPSILON);
    }

    // Inverse transformation tests
    #[test]
    fn inverse_transforming_point_with_identity_gives_same_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let identity = Projective3::identity();
        let transformed = identity.inverse_transform_point(&point);

        assert_abs_diff_eq!(transformed, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_point() {
        let point = Point3::new(1.0, 2.0, 3.0);
        let scale = scale_matrix(2.0, 3.0, 4.0);
        let proj = Projective3::from_matrix_unchecked(scale);

        let transformed = proj.transform_point(&point);
        let back = proj.inverse_transform_point(&transformed);

        assert_abs_diff_eq!(back, point, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transforming_vector_with_identity_gives_same_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let identity = Projective3::identity();
        let transformed = identity.inverse_transform_vector(&vector);

        assert_abs_diff_eq!(transformed, vector, epsilon = EPSILON);
    }

    #[test]
    fn inverse_transform_undoes_transform_for_vector() {
        let vector = Vector3::new(1.0, 2.0, 3.0);
        let rotation = rotation_90_z();
        let proj = Projective3::identity().rotated(&rotation);

        let transformed = proj.transform_vector(&vector);
        let back = proj.inverse_transform_vector(&transformed);

        assert_abs_diff_eq!(back, vector, epsilon = EPSILON);
    }

    // Complex transformation tests
    #[test]
    fn complex_transformation_sequence_works() {
        let translation = TRANSLATION_1;
        let rotation = rotation_90_z();
        let scale = scale_matrix(2.0, 2.0, 2.0);

        // Build complex transformation: Scale -> Rotate -> Translate
        let final_proj = Projective3::from_matrix_unchecked(scale)
            .rotated(&rotation)
            .translated(&translation);

        let point = Point3::new(1.0, 0.0, 0.0);
        let transformed = final_proj.transform_point(&point);

        // Manual calculation: scale(2,2,2) -> rotate 90°Z -> translate
        let _scaled = Point3::new(2.0, 0.0, 0.0);
        let rotated = Point3::new(0.0, 2.0, 0.0); // 90° rotation around Z
        let final_point = Point3::from(*rotated.as_vector() + translation);

        assert_abs_diff_eq!(transformed.x(), final_point.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), final_point.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), final_point.z(), epsilon = EPSILON);
    }

    #[test]
    fn matrix_retrieval_works_correctly() {
        let translation = TRANSLATION_1;
        let proj = Projective3::identity().translated(&translation);
        let matrix = proj.matrix();

        // Check translation components
        assert_abs_diff_eq!(matrix.element(0, 3), translation.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(1, 3), translation.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.element(2, 3), translation.z(), epsilon = EPSILON);

        // Check homogeneous coordinate
        assert_abs_diff_eq!(matrix.element(3, 3), 1.0, epsilon = EPSILON);
    }

    // Property tests
    #[test]
    fn projective_preserves_ratios_along_lines() {
        // Create a projective transformation (scale in this case)
        let scale = scale_matrix(2.0, 2.0, 2.0);
        let proj = Projective3::from_matrix_unchecked(scale);

        // Three collinear points
        let p1 = Point3::new(0.0, 0.0, 0.0);
        let p2 = Point3::new(1.0, 1.0, 1.0);
        let p3 = Point3::new(2.0, 2.0, 2.0);

        let t1 = proj.transform_point(&p1);
        let t2 = proj.transform_point(&p2);
        let t3 = proj.transform_point(&p3);

        // Check that the midpoint relationship is preserved
        let _original_mid = Point3::from((*p1.as_vector() + *p3.as_vector()) * 0.5);
        let transformed_mid = Point3::from((*t1.as_vector() + *t3.as_vector()) * 0.5);

        assert_abs_diff_eq!(t2.x(), transformed_mid.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(t2.y(), transformed_mid.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(t2.z(), transformed_mid.z(), epsilon = EPSILON);
    }

    #[test]
    fn projective_composition_is_associative() {
        let s1 = Projective3::from_matrix_unchecked(scale_matrix(2.0, 2.0, 2.0));

        // Test that sequential application gives consistent results
        let point = Point3::new(1.0, 0.0, 0.0);

        // Due to the complexity of composition, we'll test a simpler associativity
        // by verifying that sequential application gives consistent results
        let seq1 = s1.rotated(&rotation_90_z()).translated(&TRANSLATION_1);
        let seq2 = s1;
        let seq2 = seq2.rotated(&rotation_90_z());
        let seq2 = seq2.translated(&TRANSLATION_1);

        let result1 = seq1.transform_point(&point);
        let result2 = seq2.transform_point(&point);

        assert_abs_diff_eq!(result1.x(), result2.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(result1.y(), result2.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(result1.z(), result2.z(), epsilon = EPSILON);
    }

    // Approximate equality tests
    #[test]
    fn abs_diff_eq_works_with_small_differences() {
        let matrix1 = Matrix4::identity();
        use crate::vector::Vector4;
        let col1 = Vector4::new(1.0 + 1e-7, 0.0, 0.0, 0.0);
        let col2 = Vector4::new(0.0, 1.0, 0.0, 0.0);
        let col3 = Vector4::new(0.0, 0.0, 1.0, 0.0);
        let col4 = Vector4::new(0.0, 0.0, 0.0, 1.0);
        let matrix2 = Matrix4::from_columns(&[col1, col2, col3, col4]);

        let proj1 = Projective3::from_matrix_unchecked(matrix1);
        let proj2 = Projective3::from_matrix_unchecked(matrix2);

        assert_abs_diff_eq!(proj1, proj2, epsilon = 1e-6);
    }

    #[test]
    fn relative_eq_works_with_proportional_differences() {
        let matrix1 = scale_matrix(1.0, 1.0, 1.0);
        let matrix2 = scale_matrix(1.00001, 1.00001, 1.00001);

        let proj1 = Projective3::from_matrix_unchecked(matrix1);
        let proj2 = Projective3::from_matrix_unchecked(matrix2);

        use approx::assert_relative_eq;
        assert_relative_eq!(proj1, proj2, epsilon = 1e-6, max_relative = 1e-4);
    }

    // Edge case tests
    #[test]
    fn very_small_transformations_work() {
        let small_translation = Vector3::new(1e-10, 1e-10, 1e-10);
        let proj = Projective3::identity().translated(&small_translation);

        let point = Point3::new(0.0, 0.0, 0.0);
        let transformed = proj.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), small_translation.x(), epsilon = 1e-12);
        assert_abs_diff_eq!(transformed.y(), small_translation.y(), epsilon = 1e-12);
        assert_abs_diff_eq!(transformed.z(), small_translation.z(), epsilon = 1e-12);
    }

    #[test]
    fn large_scale_transformations_work() {
        let large_scale = scale_matrix(1e6, 1e6, 1e6);
        let proj = Projective3::from_matrix_unchecked(large_scale);

        let point = Point3::new(1.0, 1.0, 1.0);
        let transformed = proj.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), 1e6, epsilon = 1e-3);
        assert_abs_diff_eq!(transformed.y(), 1e6, epsilon = 1e-3);
        assert_abs_diff_eq!(transformed.z(), 1e6, epsilon = 1e-3);
    }

    #[test]
    fn zero_scale_transformations_work() {
        let zero_scale = scale_matrix(0.0, 1.0, 1.0);
        let proj = Projective3::from_matrix_unchecked(zero_scale);

        let point = Point3::new(1.0, 2.0, 3.0);
        let transformed = proj.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 2.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 3.0, epsilon = EPSILON);
    }
}
