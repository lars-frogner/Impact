//! Matrices.

use crate::{
    point::Point3,
    vector::{Vector3, Vector4},
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
pub struct Matrix3 {
    inner: nalgebra::Matrix3<f32>,
}

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix4 {
    inner: nalgebra::Matrix4<f32>,
}

impl Matrix3 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::Matrix3::identity(),
        }
    }

    #[inline]
    pub fn zeros() -> Self {
        Self {
            inner: nalgebra::Matrix3::zeros(),
        }
    }

    #[inline]
    pub fn from_diagonal(diagonal: &Vector3) -> Self {
        Self {
            inner: nalgebra::Matrix3::from_diagonal(diagonal._inner()),
        }
    }

    #[inline]
    pub fn from_columns(columns: &[Vector3; 3]) -> Self {
        Self {
            inner: nalgebra::Matrix3::from_columns(
                bytemuck::cast_slice::<_, nalgebra::Vector3<f32>>(columns),
            ),
        }
    }

    #[inline]
    pub fn inverted(&self) -> Option<Self> {
        self.inner.try_inverse().map(|inner| Self { inner })
    }

    #[inline]
    pub fn transposed(&self) -> Self {
        Self {
            inner: self.inner.transpose(),
        }
    }

    #[inline]
    pub fn negated(&self) -> Self {
        Self { inner: -self.inner }
    }

    #[inline]
    pub fn mapped(&self, f: impl FnMut(f32) -> f32) -> Self {
        Self {
            inner: self.inner.map(f),
        }
    }

    #[inline]
    pub fn element(&self, i: usize, j: usize) -> f32 {
        *self.inner.index((i, j))
    }

    #[inline]
    pub fn element_mut(&mut self, i: usize, j: usize) -> &mut f32 {
        self.inner.index_mut((i, j))
    }

    #[inline]
    pub fn column1(&self) -> Vector3 {
        Vector3::_wrap(self.inner.column(0).into_owned())
    }

    #[inline]
    pub fn column2(&self) -> Vector3 {
        Vector3::_wrap(self.inner.column(1).into_owned())
    }

    #[inline]
    pub fn column3(&self) -> Vector3 {
        Vector3::_wrap(self.inner.column(2).into_owned())
    }

    #[inline]
    pub fn diagonal(&self) -> Vector3 {
        Vector3::_wrap(self.inner.diagonal())
    }

    #[inline]
    pub fn max_element(&self) -> f32 {
        self.inner.max()
    }

    #[inline]
    pub fn _wrap(inner: nalgebra::Matrix3<f32>) -> Self {
        Self { inner }
    }
}

impl_binop!(Add, add, Matrix3, Matrix3, Matrix3, |a, b| {
    Matrix3 {
        inner: a.inner + b.inner,
    }
});

impl_binop!(Sub, sub, Matrix3, Matrix3, Matrix3, |a, b| {
    Matrix3 {
        inner: a.inner - b.inner,
    }
});

impl_binop!(Mul, mul, Matrix3, Matrix3, Matrix3, |a, b| {
    Matrix3 {
        inner: a.inner * b.inner,
    }
});

impl_binop!(Mul, mul, Matrix3, Vector3, Vector3, |a, b| {
    Vector3::_wrap(a.inner * b._inner())
});

impl_binop!(Mul, mul, Matrix3, f32, Matrix3, |a, b| {
    Matrix3 {
        inner: a.inner * *b,
    }
});

impl_binop!(Mul, mul, f32, Matrix3, Matrix3, |a, b| {
    Matrix3 {
        inner: *a * b.inner,
    }
});

impl_abs_diff_eq!(Matrix3, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Matrix3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl Matrix4 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            inner: nalgebra::Matrix4::identity(),
        }
    }

    #[inline]
    pub fn zeros() -> Self {
        Self {
            inner: nalgebra::Matrix4::zeros(),
        }
    }

    #[inline]
    pub fn from_diagonal(diagonal: &Vector4) -> Self {
        Self {
            inner: nalgebra::Matrix4::from_diagonal(diagonal._inner()),
        }
    }

    #[inline]
    pub fn from_columns(columns: &[Vector4; 4]) -> Self {
        Self {
            inner: nalgebra::Matrix4::from_columns(
                bytemuck::cast_slice::<_, nalgebra::Vector4<f32>>(columns),
            ),
        }
    }

    #[inline]
    pub fn transposed(&self) -> Self {
        Self {
            inner: self.inner.transpose(),
        }
    }

    #[inline]
    pub fn negated(&self) -> Self {
        Self { inner: -self.inner }
    }

    #[inline]
    pub fn translate_transform(&mut self, translation: &Vector3) {
        self.inner.append_translation_mut(translation._inner());
    }

    #[inline]
    pub fn scale_transform(&mut self, scaling: f32) {
        self.inner.append_scaling_mut(scaling);
    }

    #[inline]
    pub fn element(&self, i: usize, j: usize) -> f32 {
        *self.inner.index((i, j))
    }

    #[inline]
    pub fn element_mut(&mut self, i: usize, j: usize) -> &mut f32 {
        self.inner.index_mut((i, j))
    }

    #[inline]
    pub fn column1(&self) -> Vector4 {
        Vector4::_wrap(self.inner.column(0).into_owned())
    }

    #[inline]
    pub fn column2(&self) -> Vector4 {
        Vector4::_wrap(self.inner.column(1).into_owned())
    }

    #[inline]
    pub fn column3(&self) -> Vector4 {
        Vector4::_wrap(self.inner.column(2).into_owned())
    }

    #[inline]
    pub fn column4(&self) -> Vector4 {
        Vector4::_wrap(self.inner.column(3).into_owned())
    }

    #[inline]
    pub fn diagonal(&self) -> Vector4 {
        Vector4::_wrap(self.inner.diagonal())
    }

    #[inline]
    pub fn inverted(&self) -> Option<Self> {
        self.inner.try_inverse().map(|inner| Self { inner })
    }

    #[inline]
    pub fn mapped(&self, f: impl FnMut(f32) -> f32) -> Self {
        Self {
            inner: self.inner.map(f),
        }
    }

    #[inline]
    pub fn linear_part(&self) -> Matrix3 {
        Matrix3::_wrap(self.inner.fixed_view::<3, 3>(0, 0).into_owned())
    }

    #[inline]
    pub fn max_element(&self) -> f32 {
        self.inner.max()
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
    pub fn _wrap(inner: nalgebra::Matrix4<f32>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn _inner(&self) -> &nalgebra::Matrix4<f32> {
        &self.inner
    }
}

impl_binop!(Add, add, Matrix4, Matrix4, Matrix4, |a, b| {
    Matrix4 {
        inner: a.inner + b.inner,
    }
});

impl_binop!(Sub, sub, Matrix4, Matrix4, Matrix4, |a, b| {
    Matrix4 {
        inner: a.inner - b.inner,
    }
});

impl_binop!(Mul, mul, Matrix4, Matrix4, Matrix4, |a, b| {
    Matrix4 {
        inner: a.inner * b.inner,
    }
});

impl_binop!(Mul, mul, Matrix4, Vector4, Vector4, |a, b| {
    Vector4::_wrap(a.inner * b._inner())
});

impl_binop!(Mul, mul, Matrix4, f32, Matrix4, |a, b| {
    Matrix4 {
        inner: a.inner * *b,
    }
});

impl_binop!(Mul, mul, f32, Matrix4, Matrix4, |a, b| {
    Matrix4 {
        inner: *a * b.inner,
    }
});

impl_abs_diff_eq!(Matrix4, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Matrix4, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl_roc_for_library_provided_primitives! {
//  Type       Pkg   Parents  Module   Roc name  Postfix  Precision
    Matrix3 => core, None,    Matrix3, Matrix3,  None,    PrecisionIrrelevant,
    Matrix4 => core, None,    Matrix4, Matrix4,  None,    PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use crate::{
        point::Point3,
        vector::{Vector3, Vector4},
    };
    use approx::assert_abs_diff_eq;

    // Test constants
    const EPSILON: f32 = 1e-6;

    // Matrix3 tests
    #[test]
    fn matrix3_identity_creates_identity_matrix() {
        let identity = Matrix3::identity();
        assert_eq!(identity.element(0, 0), 1.0);
        assert_eq!(identity.element(1, 1), 1.0);
        assert_eq!(identity.element(2, 2), 1.0);
        // Check off-diagonal elements are zero
        assert_eq!(identity.element(0, 1), 0.0);
        assert_eq!(identity.element(0, 2), 0.0);
        assert_eq!(identity.element(1, 0), 0.0);
        assert_eq!(identity.element(1, 2), 0.0);
        assert_eq!(identity.element(2, 0), 0.0);
        assert_eq!(identity.element(2, 1), 0.0);
    }

    #[test]
    fn matrix3_zeros_creates_zero_matrix() {
        let zeros = Matrix3::zeros();
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(zeros.element(i, j), 0.0);
            }
        }
    }

    #[test]
    fn matrix3_from_diagonal_works() {
        let diag = Vector3::new(2.0, 3.0, 4.0);
        let matrix = Matrix3::from_diagonal(&diag);

        assert_eq!(matrix.element(0, 0), 2.0);
        assert_eq!(matrix.element(1, 1), 3.0);
        assert_eq!(matrix.element(2, 2), 4.0);

        // Check off-diagonal elements are zero
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    assert_eq!(matrix.element(i, j), 0.0);
                }
            }
        }
    }

    #[test]
    fn matrix3_from_columns_works() {
        let col1 = Vector3::new(1.0, 2.0, 3.0);
        let col2 = Vector3::new(4.0, 5.0, 6.0);
        let col3 = Vector3::new(7.0, 8.0, 9.0);
        let columns = [col1, col2, col3];

        let matrix = Matrix3::from_columns(&columns);

        assert_eq!(matrix.element(0, 0), 1.0);
        assert_eq!(matrix.element(1, 0), 2.0);
        assert_eq!(matrix.element(2, 0), 3.0);
        assert_eq!(matrix.element(0, 1), 4.0);
        assert_eq!(matrix.element(1, 1), 5.0);
        assert_eq!(matrix.element(2, 1), 6.0);
        assert_eq!(matrix.element(0, 2), 7.0);
        assert_eq!(matrix.element(1, 2), 8.0);
        assert_eq!(matrix.element(2, 2), 9.0);
    }

    #[test]
    fn matrix3_element_access_works() {
        let mut matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));

        assert_eq!(matrix.element(0, 0), 1.0);
        assert_eq!(matrix.element(1, 1), 2.0);
        assert_eq!(matrix.element(2, 2), 3.0);

        *matrix.element_mut(0, 1) = 5.0;
        assert_eq!(matrix.element(0, 1), 5.0);
    }

    #[test]
    fn matrix3_column_extraction_works() {
        let col1 = Vector3::new(1.0, 2.0, 3.0);
        let col2 = Vector3::new(4.0, 5.0, 6.0);
        let col3 = Vector3::new(7.0, 8.0, 9.0);
        let matrix = Matrix3::from_columns(&[col1, col2, col3]);

        let extracted_col1 = matrix.column1();
        let extracted_col2 = matrix.column2();
        let extracted_col3 = matrix.column3();

        assert_eq!(extracted_col1, col1);
        assert_eq!(extracted_col2, col2);
        assert_eq!(extracted_col3, col3);
    }

    #[test]
    fn matrix3_diagonal_extraction_works() {
        let diag_vec = Vector3::new(2.0, 3.0, 4.0);
        let matrix = Matrix3::from_diagonal(&diag_vec);
        let extracted_diag = matrix.diagonal();

        assert_eq!(extracted_diag, diag_vec);
    }

    #[test]
    fn matrix3_transposed_works() {
        let col1 = Vector3::new(1.0, 2.0, 3.0);
        let col2 = Vector3::new(4.0, 5.0, 6.0);
        let col3 = Vector3::new(7.0, 8.0, 9.0);
        let matrix = Matrix3::from_columns(&[col1, col2, col3]);

        let transposed = matrix.transposed();

        // Original matrix: columns become rows in transposed
        assert_eq!(transposed.element(0, 0), 1.0);
        assert_eq!(transposed.element(0, 1), 2.0);
        assert_eq!(transposed.element(0, 2), 3.0);
        assert_eq!(transposed.element(1, 0), 4.0);
        assert_eq!(transposed.element(1, 1), 5.0);
        assert_eq!(transposed.element(1, 2), 6.0);
        assert_eq!(transposed.element(2, 0), 7.0);
        assert_eq!(transposed.element(2, 1), 8.0);
        assert_eq!(transposed.element(2, 2), 9.0);
    }

    #[test]
    fn matrix3_negated_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(2.0, -3.0, 4.0));
        let negated = matrix.negated();

        assert_eq!(negated.element(0, 0), -2.0);
        assert_eq!(negated.element(1, 1), 3.0);
        assert_eq!(negated.element(2, 2), -4.0);
    }

    #[test]
    fn matrix3_mapped_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let mapped = matrix.mapped(|x| x * 2.0);

        assert_eq!(mapped.element(0, 0), 2.0);
        assert_eq!(mapped.element(1, 1), 4.0);
        assert_eq!(mapped.element(2, 2), 6.0);
    }

    #[test]
    fn matrix3_inverted_works() {
        let identity = Matrix3::identity();
        let inverted = identity.inverted().unwrap();

        // Identity matrix is its own inverse
        for i in 0..3 {
            for j in 0..3 {
                assert_abs_diff_eq!(
                    inverted.element(i, j),
                    identity.element(i, j),
                    epsilon = EPSILON
                );
            }
        }
    }

    #[test]
    fn matrix3_inverted_returns_none_for_singular_matrix() {
        let singular = Matrix3::zeros();
        assert!(singular.inverted().is_none());
    }

    #[test]
    fn matrix3_max_element_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 5.0, 3.0));
        assert_abs_diff_eq!(matrix.max_element(), 5.0, epsilon = EPSILON);
    }

    #[test]
    fn matrix3_arithmetic_operations_work() {
        let m1 = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let m2 = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));

        let add_result = &m1 + &m2;
        assert_eq!(add_result.element(0, 0), 3.0);
        assert_eq!(add_result.element(1, 1), 5.0);
        assert_eq!(add_result.element(2, 2), 7.0);

        let sub_result = &m2 - &m1;
        assert_eq!(sub_result.element(0, 0), 1.0);
        assert_eq!(sub_result.element(1, 1), 1.0);
        assert_eq!(sub_result.element(2, 2), 1.0);

        let mul_result = &m1 * &m2;
        assert_eq!(mul_result.element(0, 0), 2.0);
        assert_eq!(mul_result.element(1, 1), 6.0);
        assert_eq!(mul_result.element(2, 2), 12.0);
    }

    #[test]
    fn matrix3_vector_multiplication_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));
        let vector = Vector3::new(1.0, 1.0, 1.0);

        let result = &matrix * &vector;
        assert_eq!(result.x(), 2.0);
        assert_eq!(result.y(), 3.0);
        assert_eq!(result.z(), 4.0);
    }

    #[test]
    fn matrix3_scalar_multiplication_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));

        let mul_right = &matrix * 2.0;
        assert_eq!(mul_right.element(0, 0), 2.0);
        assert_eq!(mul_right.element(1, 1), 4.0);
        assert_eq!(mul_right.element(2, 2), 6.0);

        let mul_left = 3.0 * &matrix;
        assert_eq!(mul_left.element(0, 0), 3.0);
        assert_eq!(mul_left.element(1, 1), 6.0);
        assert_eq!(mul_left.element(2, 2), 9.0);
    }

    // Matrix4 tests
    #[test]
    fn matrix4_identity_creates_identity_matrix() {
        let identity = Matrix4::identity();
        for i in 0..4 {
            for j in 0..4 {
                if i == j {
                    assert_eq!(identity.element(i, j), 1.0);
                } else {
                    assert_eq!(identity.element(i, j), 0.0);
                }
            }
        }
    }

    #[test]
    fn matrix4_zeros_creates_zero_matrix() {
        let zeros = Matrix4::zeros();
        for i in 0..4 {
            for j in 0..4 {
                assert_eq!(zeros.element(i, j), 0.0);
            }
        }
    }

    #[test]
    fn matrix4_from_diagonal_works() {
        let diag = Vector4::new(2.0, 3.0, 4.0, 5.0);
        let matrix = Matrix4::from_diagonal(&diag);

        assert_eq!(matrix.element(0, 0), 2.0);
        assert_eq!(matrix.element(1, 1), 3.0);
        assert_eq!(matrix.element(2, 2), 4.0);
        assert_eq!(matrix.element(3, 3), 5.0);

        // Check off-diagonal elements are zero
        for i in 0..4 {
            for j in 0..4 {
                if i != j {
                    assert_eq!(matrix.element(i, j), 0.0);
                }
            }
        }
    }

    #[test]
    fn matrix4_from_columns_works() {
        let col1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4::new(13.0, 14.0, 15.0, 16.0);
        let columns = [col1, col2, col3, col4];

        let matrix = Matrix4::from_columns(&columns);

        for i in 0..4 {
            for j in 0..4 {
                let expected = (j * 4 + i + 1) as f32;
                assert_eq!(matrix.element(i, j), expected);
            }
        }
    }

    #[test]
    fn matrix4_element_access_works() {
        let mut matrix = Matrix4::from_diagonal(&Vector4::new(1.0, 2.0, 3.0, 4.0));

        assert_eq!(matrix.element(0, 0), 1.0);
        assert_eq!(matrix.element(1, 1), 2.0);
        assert_eq!(matrix.element(2, 2), 3.0);
        assert_eq!(matrix.element(3, 3), 4.0);

        *matrix.element_mut(0, 1) = 5.0;
        assert_eq!(matrix.element(0, 1), 5.0);
    }

    #[test]
    fn matrix4_column_extraction_works() {
        let col1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4::from_columns(&[col1, col2, col3, col4]);

        let extracted_col1 = matrix.column1();
        let extracted_col2 = matrix.column2();
        let extracted_col3 = matrix.column3();
        let extracted_col4 = matrix.column4();

        assert_eq!(extracted_col1, col1);
        assert_eq!(extracted_col2, col2);
        assert_eq!(extracted_col3, col3);
        assert_eq!(extracted_col4, col4);
    }

    #[test]
    fn matrix4_diagonal_extraction_works() {
        let diag_vec = Vector4::new(2.0, 3.0, 4.0, 5.0);
        let matrix = Matrix4::from_diagonal(&diag_vec);
        let extracted_diag = matrix.diagonal();

        assert_eq!(extracted_diag, diag_vec);
    }

    #[test]
    fn matrix4_transposed_works() {
        let col1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4::from_columns(&[col1, col2, col3, col4]);

        let transposed = matrix.transposed();

        // Original columns become rows in transposed
        assert_eq!(transposed.element(0, 0), 1.0);
        assert_eq!(transposed.element(0, 1), 2.0);
        assert_eq!(transposed.element(0, 2), 3.0);
        assert_eq!(transposed.element(0, 3), 4.0);
    }

    #[test]
    fn matrix4_negated_works() {
        let matrix = Matrix4::from_diagonal(&Vector4::new(2.0, -3.0, 4.0, -5.0));
        let negated = matrix.negated();

        assert_eq!(negated.element(0, 0), -2.0);
        assert_eq!(negated.element(1, 1), 3.0);
        assert_eq!(negated.element(2, 2), -4.0);
        assert_eq!(negated.element(3, 3), 5.0);
    }

    #[test]
    fn matrix4_mapped_works() {
        let matrix = Matrix4::from_diagonal(&Vector4::new(1.0, 2.0, 3.0, 4.0));
        let mapped = matrix.mapped(|x| x * 2.0);

        assert_eq!(mapped.element(0, 0), 2.0);
        assert_eq!(mapped.element(1, 1), 4.0);
        assert_eq!(mapped.element(2, 2), 6.0);
        assert_eq!(mapped.element(3, 3), 8.0);
    }

    #[test]
    fn matrix4_inverted_works() {
        let identity = Matrix4::identity();
        let inverted = identity.inverted().unwrap();

        // Identity matrix is its own inverse
        for i in 0..4 {
            for j in 0..4 {
                assert_abs_diff_eq!(
                    inverted.element(i, j),
                    identity.element(i, j),
                    epsilon = EPSILON
                );
            }
        }
    }

    #[test]
    fn matrix4_inverted_returns_none_for_singular_matrix() {
        let singular = Matrix4::zeros();
        assert!(singular.inverted().is_none());
    }

    #[test]
    fn matrix4_linear_part_works() {
        let col1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4::from_columns(&[col1, col2, col3, col4]);

        let linear = matrix.linear_part();

        // Linear part is the upper-left 3x3 submatrix
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(linear.element(i, j), matrix.element(i, j));
            }
        }
    }

    #[test]
    fn matrix4_max_element_works() {
        let matrix = Matrix4::from_diagonal(&Vector4::new(1.0, 7.0, 3.0, 2.0));
        assert_abs_diff_eq!(matrix.max_element(), 7.0, epsilon = EPSILON);
    }

    #[test]
    fn matrix4_transform_operations_work() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(1.0, 2.0, 3.0);

        matrix.translate_transform(&translation);
        // Translation should be in the last column
        assert_eq!(matrix.element(0, 3), 1.0);
        assert_eq!(matrix.element(1, 3), 2.0);
        assert_eq!(matrix.element(2, 3), 3.0);

        let mut scale_matrix = Matrix4::identity();
        scale_matrix.scale_transform(2.0);
        // Scaling affects the diagonal
        assert_eq!(scale_matrix.element(0, 0), 2.0);
        assert_eq!(scale_matrix.element(1, 1), 2.0);
        assert_eq!(scale_matrix.element(2, 2), 2.0);
    }

    #[test]
    fn matrix4_transform_point_works() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(1.0, 2.0, 3.0);
        matrix.translate_transform(&translation);

        let point = Point3::new(0.0, 0.0, 0.0);
        let transformed = matrix.transform_point(&point);

        assert_eq!(transformed.x(), 1.0);
        assert_eq!(transformed.y(), 2.0);
        assert_eq!(transformed.z(), 3.0);
    }

    #[test]
    fn matrix4_transform_vector_works() {
        let mut matrix = Matrix4::identity();
        matrix.scale_transform(2.0);

        let vector = Vector3::new(1.0, 2.0, 3.0);
        let transformed = matrix.transform_vector(&vector);

        assert_eq!(transformed.x(), 2.0);
        assert_eq!(transformed.y(), 4.0);
        assert_eq!(transformed.z(), 6.0);
    }

    #[test]
    fn matrix4_arithmetic_operations_work() {
        let m1 = Matrix4::from_diagonal(&Vector4::new(1.0, 2.0, 3.0, 4.0));
        let m2 = Matrix4::from_diagonal(&Vector4::new(2.0, 3.0, 4.0, 5.0));

        let add_result = &m1 + &m2;
        assert_eq!(add_result.element(0, 0), 3.0);
        assert_eq!(add_result.element(1, 1), 5.0);
        assert_eq!(add_result.element(2, 2), 7.0);
        assert_eq!(add_result.element(3, 3), 9.0);

        let sub_result = &m2 - &m1;
        assert_eq!(sub_result.element(0, 0), 1.0);
        assert_eq!(sub_result.element(1, 1), 1.0);
        assert_eq!(sub_result.element(2, 2), 1.0);
        assert_eq!(sub_result.element(3, 3), 1.0);

        let mul_result = &m1 * &m2;
        assert_eq!(mul_result.element(0, 0), 2.0);
        assert_eq!(mul_result.element(1, 1), 6.0);
        assert_eq!(mul_result.element(2, 2), 12.0);
        assert_eq!(mul_result.element(3, 3), 20.0);
    }

    #[test]
    fn matrix4_vector_multiplication_works() {
        let matrix = Matrix4::from_diagonal(&Vector4::new(2.0, 3.0, 4.0, 5.0));
        let vector = Vector4::new(1.0, 1.0, 1.0, 1.0);

        let result = &matrix * &vector;
        assert_eq!(result.x(), 2.0);
        assert_eq!(result.y(), 3.0);
        assert_eq!(result.z(), 4.0);
        assert_eq!(result.w(), 5.0);
    }

    #[test]
    fn matrix4_scalar_multiplication_works() {
        let matrix = Matrix4::from_diagonal(&Vector4::new(1.0, 2.0, 3.0, 4.0));

        let mul_right = &matrix * 2.0;
        assert_eq!(mul_right.element(0, 0), 2.0);
        assert_eq!(mul_right.element(1, 1), 4.0);
        assert_eq!(mul_right.element(2, 2), 6.0);
        assert_eq!(mul_right.element(3, 3), 8.0);

        let mul_left = 3.0 * &matrix;
        assert_eq!(mul_left.element(0, 0), 3.0);
        assert_eq!(mul_left.element(1, 1), 6.0);
        assert_eq!(mul_left.element(2, 2), 9.0);
        assert_eq!(mul_left.element(3, 3), 12.0);
    }

    #[test]
    fn matrix_operations_with_different_reference_combinations_work() {
        let m1 = Matrix3::identity();
        let m2 = Matrix3::identity();

        // Test all combinations of reference/owned for binary operations
        let _result1 = &m1 + &m2; // ref + ref
        let _result2 = &m1 + m2; // ref + owned
        let _result3 = m1 + &m2; // owned + ref
        let _result4 = m1 + m2; // owned + owned

        // Recreate since they were moved
        let m1 = Matrix3::identity();
        let _result5 = 2.0 * &m1; // scalar * ref
        let _result6 = 2.0 * m1; // scalar * owned

        let m1 = Matrix3::identity();
        let _result7 = &m1 * 2.0; // ref * scalar
        let _result8 = m1 * 2.0; // owned * scalar
    }

    #[test]
    fn matrix_arithmetic_maintains_precision() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(0.1, 0.2, 0.3));
        let doubled = &matrix * 2.0;
        let halved = &doubled * 0.5;

        for i in 0..3 {
            for j in 0..3 {
                assert_abs_diff_eq!(
                    halved.element(i, j),
                    matrix.element(i, j),
                    epsilon = EPSILON
                );
            }
        }
    }

    #[test]
    fn matrix_identity_properties_hold() {
        let identity3 = Matrix3::identity();
        let test_matrix3 = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));

        let left_mult = &identity3 * &test_matrix3;
        let right_mult = &test_matrix3 * &identity3;

        // I * M = M * I = M
        assert_eq!(left_mult, test_matrix3);
        assert_eq!(right_mult, test_matrix3);

        // Same for Matrix4
        let identity4 = Matrix4::identity();
        let test_matrix4 = Matrix4::from_diagonal(&Vector4::new(2.0, 3.0, 4.0, 5.0));

        let left_mult4 = &identity4 * &test_matrix4;
        let right_mult4 = &test_matrix4 * &identity4;

        assert_eq!(left_mult4, test_matrix4);
        assert_eq!(right_mult4, test_matrix4);
    }

    #[test]
    fn matrix_transpose_is_involutive() {
        let matrix = Matrix3::from_columns(&[
            Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(4.0, 5.0, 6.0),
            Vector3::new(7.0, 8.0, 9.0),
        ]);

        let double_transposed = matrix.transposed().transposed();

        // (A^T)^T = A
        assert_eq!(double_transposed, matrix);
    }

    #[test]
    fn matrix_inversion_properties_hold() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));
        let inverse = matrix.inverted().unwrap();
        let product = &matrix * &inverse;

        // M * M^-1 should be approximately identity
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert_abs_diff_eq!(product.element(i, j), expected, epsilon = EPSILON);
            }
        }
    }

    #[test]
    fn matrix_transform_composition_works() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let scale = 2.0;

        matrix.translate_transform(&translation);
        matrix.scale_transform(scale);

        let point = Point3::new(1.0, 1.0, 1.0);
        let transformed = matrix.transform_point(&point);

        // Transform order is translate first, then scale: ((1,1,1) + (1,2,3)) * 2 = (4,6,8)
        assert_abs_diff_eq!(transformed.x(), 4.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 6.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 8.0, epsilon = EPSILON);
    }

    #[test]
    fn matrix_vector_vs_point_transform_difference() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(5.0, 5.0, 5.0);
        matrix.translate_transform(&translation);

        let vector = Vector3::new(1.0, 1.0, 1.0);
        let point = Point3::new(1.0, 1.0, 1.0);

        let transformed_vector = matrix.transform_vector(&vector);
        let transformed_point = matrix.transform_point(&point);

        // Vector should not be affected by translation
        assert_eq!(transformed_vector, vector);

        // Point should be translated
        assert_eq!(transformed_point.x(), 6.0);
        assert_eq!(transformed_point.y(), 6.0);
        assert_eq!(transformed_point.z(), 6.0);
    }
}
