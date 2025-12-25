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
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Projective3 {
    matrix: Matrix4,
}

/// A projective transform backed by a 4x4 homogeneous matrix, aligned to 16
/// bytes.
///
/// The matrix columns are stored in 128-bit SIMD registers for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned
/// [`Projective3`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{matrix::Matrix4A, point::Point3A, vector::Vector4A};
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

    #[test]
    fn creating_projective3a_from_matrix_stores_matrix() {
        let matrix = test_scale_matrix_aligned();
        let proj = Projective3A::from_matrix_unchecked(matrix);

        assert_abs_diff_eq!(*proj.matrix(), matrix, epsilon = EPSILON);
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
}
