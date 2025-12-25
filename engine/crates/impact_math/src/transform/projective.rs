//! Projective transforms.

use crate::{
    matrix::{Matrix4, Matrix4A},
    point::Point3A,
};
use bytemuck::{Pod, Zeroable};

/// A projective transform backed by a 4x4 homogeneous matrix.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Projective3A`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct Projective3 {
    matrix: Matrix4,
}

/// A projective transform backed by a 4x4 homogeneous matrix.
///
/// The matrix columns are stored in 128-bit SIMD registers for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned
/// [`Projective3`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct Projective3A {
    matrix: Matrix4A,
}

impl Projective3 {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_matrix_unchecked(Matrix4::identity())
    }

    /// Creates a projective transform corresponding to the given 4x4
    /// homogeneous matrix. The matrix is assumed to represent a valid
    /// projective transform.
    #[inline]
    pub const fn from_matrix_unchecked(matrix: Matrix4) -> Self {
        Self { matrix }
    }

    /// Returns the projection matrix.
    #[inline]
    pub const fn matrix(&self) -> &Matrix4 {
        &self.matrix
    }

    /// Converts the transform to the 16-byte aligned SIMD-friendly
    /// [`Projective3A`].
    #[inline]
    pub fn aligned(&self) -> Projective3A {
        Projective3A::from_matrix_unchecked(self.matrix().aligned())
    }
}

impl_abs_diff_eq!(Projective3, |a, b, epsilon| {
    a.matrix.abs_diff_eq(&b.matrix, epsilon)
});

impl_relative_eq!(Projective3, |a, b, epsilon, max_relative| {
    a.matrix.relative_eq(&b.matrix, epsilon, max_relative)
});

impl Default for Projective3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Projective3A {
    /// Creates the identity transform.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_matrix_unchecked(Matrix4A::identity())
    }

    /// Creates a projective transform corresponding to the given 4x4
    /// homogeneous matrix. The matrix is assumed to represent a valid
    /// projective transform.
    #[inline]
    pub const fn from_matrix_unchecked(matrix: Matrix4A) -> Self {
        Self { matrix }
    }

    /// Returns the projection matrix.
    #[inline]
    pub const fn matrix(&self) -> &Matrix4A {
        &self.matrix
    }

    /// Computes the inverse of this projective transform.
    #[inline]
    pub fn inverted(&self) -> Self {
        Self::from_matrix_unchecked(self.matrix.inverted())
    }

    /// Projects the given point by applying the matrix and performing
    /// perspective division.
    #[inline]
    pub fn project_point(&self, point: &Point3A) -> Point3A {
        self.matrix.project_point(point)
    }

    /// Converts the transform to the 4-byte aligned cache-friendly
    /// [`Projective3`].
    #[inline]
    pub fn unaligned(&self) -> Projective3 {
        Projective3::from_matrix_unchecked(self.matrix().unaligned())
    }
}

impl_abs_diff_eq!(Projective3A, |a, b, epsilon| {
    a.matrix.abs_diff_eq(&b.matrix, epsilon)
});

impl_relative_eq!(Projective3A, |a, b, epsilon, max_relative| {
    a.matrix.relative_eq(&b.matrix, epsilon, max_relative)
});

impl Default for Projective3A {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        matrix::Matrix4,
        matrix::Matrix4A,
        point::Point3A,
        vector::{Vector4, Vector4A},
    };
    use approx::assert_abs_diff_eq;

    // Test constants
    const EPSILON: f32 = 1e-6;

    // Helper function to create a simple scaling matrix (aligned)
    fn test_scale_matrix_aligned() -> Matrix4A {
        Matrix4A::from_columns(
            Vector4A::new(2.0, 0.0, 0.0, 0.0),
            Vector4A::new(0.0, 3.0, 0.0, 0.0),
            Vector4A::new(0.0, 0.0, 4.0, 0.0),
            Vector4A::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    // Helper function to create a simple scaling matrix (unaligned)
    fn test_scale_matrix_unaligned() -> Matrix4 {
        Matrix4::from_columns(
            Vector4::new(2.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 3.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 4.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        )
    }

    // Helper function to create a simple perspective projection matrix
    fn test_perspective_matrix() -> Matrix4A {
        // Simple perspective projection with w = z
        Matrix4A::from_columns(
            Vector4A::new(1.0, 0.0, 0.0, 0.0),
            Vector4A::new(0.0, 1.0, 0.0, 0.0),
            Vector4A::new(0.0, 0.0, 1.0, 1.0),
            Vector4A::new(0.0, 0.0, 0.0, 0.0),
        )
    }

    // Projective3 (unaligned) tests
    #[test]
    fn creating_identity_projective3_gives_identity_matrix() {
        let proj = Projective3::identity();

        assert_abs_diff_eq!(*proj.matrix(), Matrix4::identity(), epsilon = EPSILON);
    }

    #[test]
    fn creating_projective3_from_matrix_stores_matrix() {
        let matrix = test_scale_matrix_unaligned();
        let proj = Projective3::from_matrix_unchecked(matrix);

        assert_abs_diff_eq!(*proj.matrix(), matrix, epsilon = EPSILON);
    }

    #[test]
    fn projective3_matrix_accessor_works() {
        let matrix = test_scale_matrix_unaligned();
        let proj = Projective3::from_matrix_unchecked(matrix);

        let retrieved = proj.matrix();
        assert_abs_diff_eq!(*retrieved, matrix, epsilon = EPSILON);
    }

    #[test]
    fn converting_projective3_to_aligned_works() {
        let matrix = test_scale_matrix_unaligned();
        let proj3 = Projective3::from_matrix_unchecked(matrix);
        let proj3a = proj3.aligned();

        assert_abs_diff_eq!(*proj3a.matrix(), matrix.aligned(), epsilon = EPSILON);
    }

    #[test]
    fn projective3_default_is_identity() {
        let default_proj = Projective3::default();
        let identity_proj = Projective3::identity();

        assert_abs_diff_eq!(default_proj, identity_proj, epsilon = EPSILON);
    }

    // Projective3A (aligned) tests
    #[test]
    fn creating_identity_projective3a_gives_identity_matrix() {
        let proj = Projective3A::identity();

        assert_abs_diff_eq!(*proj.matrix(), Matrix4A::identity(), epsilon = EPSILON);
    }

    #[test]
    fn creating_projective3a_from_matrix_stores_matrix() {
        let matrix = test_scale_matrix_aligned();
        let proj = Projective3A::from_matrix_unchecked(matrix);

        assert_abs_diff_eq!(*proj.matrix(), matrix, epsilon = EPSILON);
    }

    #[test]
    fn projective3a_matrix_accessor_works() {
        let matrix = test_scale_matrix_aligned();
        let proj = Projective3A::from_matrix_unchecked(matrix);

        let retrieved = proj.matrix();
        assert_abs_diff_eq!(*retrieved, matrix, epsilon = EPSILON);
    }

    #[test]
    fn projective3a_default_is_identity() {
        let default_proj = Projective3A::default();
        let identity_proj = Projective3A::identity();

        assert_abs_diff_eq!(default_proj, identity_proj, epsilon = EPSILON);
    }

    #[test]
    fn inverting_identity_gives_identity() {
        let identity = Projective3A::identity();
        let inverted = identity.inverted();

        assert_abs_diff_eq!(inverted, identity, epsilon = EPSILON);
    }

    #[test]
    fn inverting_scale_matrix_gives_inverse_scale() {
        let scale_matrix = test_scale_matrix_aligned();
        let proj = Projective3A::from_matrix_unchecked(scale_matrix);
        let inverted = proj.inverted();

        // Multiplying with inverse should give identity
        let result = scale_matrix * *inverted.matrix();
        assert_abs_diff_eq!(result, Matrix4A::identity(), epsilon = EPSILON);
    }

    #[test]
    fn inverting_twice_gives_original() {
        let matrix = test_scale_matrix_aligned();
        let proj = Projective3A::from_matrix_unchecked(matrix);
        let double_inverted = proj.inverted().inverted();

        assert_abs_diff_eq!(*double_inverted.matrix(), matrix, epsilon = EPSILON);
    }

    #[test]
    fn projecting_point_with_identity_gives_same_point() {
        let identity = Projective3A::identity();
        let point = Point3A::new(1.0, 2.0, 3.0);

        let projected = identity.project_point(&point);

        assert_abs_diff_eq!(projected, point, epsilon = EPSILON);
    }

    #[test]
    fn projecting_point_with_scale_matrix_scales_point() {
        let scale_matrix = test_scale_matrix_aligned();
        let proj = Projective3A::from_matrix_unchecked(scale_matrix);
        let point = Point3A::new(1.0, 1.0, 1.0);

        let projected = proj.project_point(&point);

        // Should be scaled by (2.0, 3.0, 4.0)
        let expected = Point3A::new(2.0, 3.0, 4.0);
        assert_abs_diff_eq!(projected, expected, epsilon = EPSILON);
    }

    #[test]
    fn projecting_point_with_perspective_performs_division() {
        let perspective_matrix = test_perspective_matrix();
        let proj = Projective3A::from_matrix_unchecked(perspective_matrix);
        let point = Point3A::new(2.0, 4.0, 2.0);

        let projected = proj.project_point(&point);

        // With w = z = 2.0, after division: (2.0/2.0, 4.0/2.0, 2.0/2.0) = (1.0, 2.0, 1.0)
        let expected = Point3A::new(1.0, 2.0, 1.0);
        assert_abs_diff_eq!(projected, expected, epsilon = EPSILON);
    }

    #[test]
    fn converting_projective3a_to_unaligned_works() {
        let matrix = test_scale_matrix_aligned();
        let proj3a = Projective3A::from_matrix_unchecked(matrix);
        let proj3 = proj3a.unaligned();

        assert_abs_diff_eq!(*proj3.matrix(), matrix.unaligned(), epsilon = EPSILON);
    }

    #[test]
    fn round_trip_conversion_preserves_values() {
        let original_matrix = test_scale_matrix_aligned();
        let proj3a = Projective3A::from_matrix_unchecked(original_matrix);
        let proj3 = proj3a.unaligned();
        let back_to_aligned = proj3.aligned();

        assert_abs_diff_eq!(
            *back_to_aligned.matrix(),
            original_matrix,
            epsilon = EPSILON
        );
    }

    // Edge case tests
    #[test]
    fn projecting_origin_with_scale_gives_origin() {
        let scale_matrix = test_scale_matrix_aligned();
        let proj = Projective3A::from_matrix_unchecked(scale_matrix);
        let origin = Point3A::new(0.0, 0.0, 0.0);

        let projected = proj.project_point(&origin);

        assert_abs_diff_eq!(projected, origin, epsilon = EPSILON);
    }

    #[test]
    fn projecting_with_negative_scale_works() {
        let neg_scale_matrix = Matrix4A::from_columns(
            Vector4A::new(-1.0, 0.0, 0.0, 0.0),
            Vector4A::new(0.0, -1.0, 0.0, 0.0),
            Vector4A::new(0.0, 0.0, -1.0, 0.0),
            Vector4A::new(0.0, 0.0, 0.0, 1.0),
        );
        let proj = Projective3A::from_matrix_unchecked(neg_scale_matrix);
        let point = Point3A::new(1.0, 2.0, 3.0);

        let projected = proj.project_point(&point);

        let expected = Point3A::new(-1.0, -2.0, -3.0);
        assert_abs_diff_eq!(projected, expected, epsilon = EPSILON);
    }
}
