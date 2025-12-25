//! Matrices.

use crate::{
    point::Point3A,
    quaternion::UnitQuaternionA,
    vector::{Vector3, Vector3A, Vector4, Vector4A},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;
use std::ops::Mul;

/// A 3x3 matrix.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Matrix3A`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix3 {
    column_1: Vector3,
    column_2: Vector3,
    column_3: Vector3,
}

/// A 3x3 matrix aligned to 16 bytes.
///
/// The columns are stored in 128-bit SIMD registers for efficient computation.
/// That leads to an extra 12 bytes in size (4 per column) and 16-byte
/// alignment. For cache-friendly storage, prefer [`Matrix3`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix3A {
    inner: glam::Mat3A,
}

/// A 4x4 vector.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`Matrix4A`].
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix4 {
    column_1: Vector4,
    column_2: Vector4,
    column_3: Vector4,
    column_4: Vector4,
}

/// A 4x4 matrix aligned to 16 bytes.
///
/// The columns are stored in 128-bit SIMD registers for efficient computation.
/// That leads to an alignment of 16 bytes. For padding-free storage together
/// with smaller types, prefer the 4-byte aligned [`Matrix4`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix4A {
    inner: glam::Mat4,
}

impl Matrix3 {
    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_columns(Vector3::unit_x(), Vector3::unit_y(), Vector3::unit_z())
    }

    /// Creates a matrix with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::from_columns(Vector3::zeros(), Vector3::zeros(), Vector3::zeros())
    }

    /// Creates a diagonal matrix with the given vector as the diagonal.
    #[inline]
    pub const fn from_diagonal(diagonal: &Vector3) -> Self {
        let mut m = Self::zeros();
        *m.column_1.x_mut() = diagonal.x();
        *m.column_2.y_mut() = diagonal.y();
        *m.column_3.z_mut() = diagonal.z();
        m
    }

    /// Creates a matrix with the given columns.
    #[inline]
    pub const fn from_columns(column_1: Vector3, column_2: Vector3, column_3: Vector3) -> Self {
        Self {
            column_1,
            column_2,
            column_3,
        }
    }

    /// The first column of the matrix.
    #[inline]
    pub const fn column_1(&self) -> &Vector3 {
        &self.column_1
    }

    /// The second column of the matrix.
    #[inline]
    pub const fn column_2(&self) -> &Vector3 {
        &self.column_2
    }

    /// The third column of the matrix.
    #[inline]
    pub const fn column_3(&self) -> &Vector3 {
        &self.column_3
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub const fn set_column_1(&mut self, column: Vector3) {
        self.column_1 = column;
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub const fn set_column_2(&mut self, column: Vector3) {
        self.column_2 = column;
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub const fn set_column_3(&mut self, column: Vector3) {
        self.column_3 = column;
    }

    /// Converts the matrix to the 16-byte aligned SIMD-friendly [`Matrix3A`].
    #[inline]
    pub fn aligned(&self) -> Matrix3A {
        Matrix3A::from_columns(
            self.column_1().aligned(),
            self.column_2().aligned(),
            self.column_3().aligned(),
        )
    }

    #[inline]
    pub(crate) const fn from_glam(matrix: glam::Mat3) -> Self {
        Self::from_columns(
            Vector3::from_glam(matrix.x_axis),
            Vector3::from_glam(matrix.y_axis),
            Vector3::from_glam(matrix.z_axis),
        )
    }
}

impl_abs_diff_eq!(Matrix3, |a, b, epsilon| {
    a.column_1.abs_diff_eq(&b.column_1, epsilon)
        && a.column_2.abs_diff_eq(&b.column_2, epsilon)
        && a.column_3.abs_diff_eq(&b.column_3, epsilon)
});

impl_relative_eq!(Matrix3, |a, b, epsilon, max_relative| {
    a.column_1.relative_eq(&b.column_1, epsilon, max_relative)
        && a.column_2.relative_eq(&b.column_2, epsilon, max_relative)
        && a.column_3.relative_eq(&b.column_3, epsilon, max_relative)
});

impl Matrix3A {
    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::wrap(glam::Mat3A::IDENTITY)
    }

    /// Creates a matrix with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::wrap(glam::Mat3A::ZERO)
    }

    /// Creates a diagonal matrix with the given vector as the diagonal.
    #[inline]
    pub const fn from_diagonal(diagonal: &Vector3) -> Self {
        Self::wrap(glam::Mat3A::from_diagonal(glam::Vec3::new(
            diagonal.x(),
            diagonal.y(),
            diagonal.z(),
        )))
    }

    /// Creates a matrix with the given columns.
    #[inline]
    pub const fn from_columns(column_1: Vector3A, column_2: Vector3A, column_3: Vector3A) -> Self {
        Self::wrap(glam::Mat3A::from_cols(
            column_1.unwrap(),
            column_2.unwrap(),
            column_3.unwrap(),
        ))
    }

    /// The first column of the matrix.
    #[inline]
    pub fn column_1(&self) -> &Vector3A {
        bytemuck::cast_ref(&self.inner.x_axis)
    }

    /// The second column of the matrix.
    #[inline]
    pub fn column_2(&self) -> &Vector3A {
        bytemuck::cast_ref(&self.inner.y_axis)
    }

    /// The third column of the matrix.
    #[inline]
    pub fn column_3(&self) -> &Vector3A {
        bytemuck::cast_ref(&self.inner.z_axis)
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub fn set_column_1(&mut self, column: Vector3A) {
        self.inner.x_axis = column.unwrap();
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub fn set_column_2(&mut self, column: Vector3A) {
        self.inner.y_axis = column.unwrap();
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub fn set_column_3(&mut self, column: Vector3A) {
        self.inner.z_axis = column.unwrap();
    }

    /// Returns the element at row `i` and column `j`.
    ///
    /// # Panics
    /// If the indices are outside the matrix.
    #[inline]
    pub fn element(&self, i: usize, j: usize) -> f32 {
        let m = &self.inner;
        match j {
            0 => m.x_axis[i],
            1 => m.y_axis[i],
            2 => m.z_axis[i],
            _ => panic!("index out of bounds"),
        }
    }

    /// Returns a mutable reference to the element at row `i` and column `j`.
    ///
    /// # Panics
    /// If the indices are outside the matrix.
    #[inline]
    pub fn element_mut(&mut self, i: usize, j: usize) -> &mut f32 {
        let m = &mut self.inner;
        match j {
            0 => &mut m.x_axis[i],
            1 => &mut m.y_axis[i],
            2 => &mut m.z_axis[i],
            _ => panic!("index out of bounds"),
        }
    }

    /// Returns the inverse of this matrix. If the matrix is not invertible, the
    /// result will be non-finite.
    #[inline]
    pub fn inverted(&self) -> Self {
        Self::wrap(self.inner.inverse())
    }

    /// Returns the transpose of this matrix.
    #[inline]
    pub fn transposed(&self) -> Self {
        Self::wrap(self.inner.transpose())
    }

    /// Returns a matrix with the given closure applied to each element.
    #[inline]
    pub fn mapped(&self, mut f: impl FnMut(f32) -> f32) -> Self {
        let x = self.inner.x_axis;
        let y = self.inner.y_axis;
        let z = self.inner.z_axis;
        Self::wrap(glam::Mat3A::from_cols(
            glam::Vec3A::new(f(x.x), f(x.y), f(x.z)),
            glam::Vec3A::new(f(y.x), f(y.y), f(y.z)),
            glam::Vec3A::new(f(z.x), f(z.y), f(z.z)),
        ))
    }

    /// Returns the diagonal of this matrix as a vector.
    #[inline]
    pub fn diagonal(&self) -> Vector3A {
        let m = &self.inner;
        Vector3A::new(m.x_axis.x, m.y_axis.y, m.z_axis.z)
    }

    /// Returns the smallest element in the matrix.
    #[inline]
    pub fn min_element(&self) -> f32 {
        let m = &self.inner;
        m.x_axis
            .min_element()
            .min(m.y_axis.min_element())
            .min(m.z_axis.min_element())
    }

    /// Returns the largest element in the matrix.
    #[inline]
    pub fn max_element(&self) -> f32 {
        let m = &self.inner;
        m.x_axis
            .max_element()
            .max(m.y_axis.max_element())
            .max(m.z_axis.max_element())
    }

    /// Converts the matrix to the 4-byte aligned cache-friendly [`Matrix3`].
    #[inline]
    pub fn unaligned(&self) -> Matrix3 {
        Matrix3::from_columns(
            self.column_1().unaligned(),
            self.column_2().unaligned(),
            self.column_3().unaligned(),
        )
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Mat3A) -> Self {
        Self { inner }
    }
}

impl_binop!(Add, add, Matrix3A, Matrix3A, Matrix3A, |a, b| {
    Matrix3A::wrap(a.inner.add_mat3(&b.inner))
});

impl_binop!(Sub, sub, Matrix3A, Matrix3A, Matrix3A, |a, b| {
    Matrix3A::wrap(a.inner.sub_mat3(&b.inner))
});

impl_binop!(Mul, mul, Matrix3A, Matrix3A, Matrix3A, |a, b| {
    Matrix3A::wrap(a.inner.mul_mat3(&b.inner))
});

impl_binop!(Mul, mul, Matrix3A, Vector3A, Vector3A, |a, b| {
    Vector3A::wrap(a.inner.mul_vec3a(b.unwrap()))
});

impl_binop!(Mul, mul, Matrix3A, f32, Matrix3A, |a, b| {
    Matrix3A::wrap(a.inner.mul_scalar(*b))
});

impl_binop!(Mul, mul, f32, Matrix3A, Matrix3A, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Matrix3A, f32, Matrix3A, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, Matrix3A, Matrix3A, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Matrix3A, Matrix3A, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix3A, Matrix3A, |a, b| {
    a.inner.mul_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix3A, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Matrix3A, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Matrix3A, Matrix3A, |val| {
    Matrix3A::wrap(val.inner.neg())
});

impl_abs_diff_eq!(Matrix3A, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Matrix3A, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl Matrix4 {
    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_columns(
            Vector4::unit_x(),
            Vector4::unit_y(),
            Vector4::unit_z(),
            Vector4::unit_w(),
        )
    }

    /// Creates a matrix with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::from_columns(
            Vector4::zeros(),
            Vector4::zeros(),
            Vector4::zeros(),
            Vector4::zeros(),
        )
    }

    /// Creates a diagonal matrix with the given vector as the diagonal.
    #[inline]
    pub const fn from_diagonal(diagonal: &Vector4) -> Self {
        let mut m = Self::zeros();
        *m.column_1.x_mut() = diagonal.x();
        *m.column_2.y_mut() = diagonal.y();
        *m.column_3.z_mut() = diagonal.z();
        *m.column_4.w_mut() = diagonal.w();
        m
    }

    /// Creates a matrix with the given columns.
    #[inline]
    pub const fn from_columns(
        column_1: Vector4,
        column_2: Vector4,
        column_3: Vector4,
        column_4: Vector4,
    ) -> Self {
        Self {
            column_1,
            column_2,
            column_3,
            column_4,
        }
    }

    /// The first column of the matrix.
    #[inline]
    pub const fn column_1(&self) -> &Vector4 {
        &self.column_1
    }

    /// The second column of the matrix.
    #[inline]
    pub const fn column_2(&self) -> &Vector4 {
        &self.column_2
    }

    /// The third column of the matrix.
    #[inline]
    pub const fn column_3(&self) -> &Vector4 {
        &self.column_3
    }

    /// The fourth column of the matrix.
    #[inline]
    pub const fn column_4(&self) -> &Vector4 {
        &self.column_4
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub const fn set_column_1(&mut self, column: Vector4) {
        self.column_1 = column;
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub const fn set_column_2(&mut self, column: Vector4) {
        self.column_2 = column;
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub const fn set_column_3(&mut self, column: Vector4) {
        self.column_3 = column;
    }

    /// Sets the fourth column of the matrix to the given column.
    #[inline]
    pub const fn set_column_4(&mut self, column: Vector4) {
        self.column_4 = column;
    }

    /// Converts the matrix to the 16-byte aligned SIMD-friendly [`Matrix4A`].
    #[inline]
    pub fn aligned(&self) -> Matrix4A {
        Matrix4A::from_columns(
            self.column_1().aligned(),
            self.column_2().aligned(),
            self.column_3().aligned(),
            self.column_4().aligned(),
        )
    }
}

impl_abs_diff_eq!(Matrix4, |a, b, epsilon| {
    a.column_1.abs_diff_eq(&b.column_1, epsilon)
        && a.column_2.abs_diff_eq(&b.column_2, epsilon)
        && a.column_3.abs_diff_eq(&b.column_3, epsilon)
        && a.column_4.abs_diff_eq(&b.column_4, epsilon)
});

impl_relative_eq!(Matrix4, |a, b, epsilon, max_relative| {
    a.column_1.relative_eq(&b.column_1, epsilon, max_relative)
        && a.column_2.relative_eq(&b.column_2, epsilon, max_relative)
        && a.column_3.relative_eq(&b.column_3, epsilon, max_relative)
        && a.column_4.relative_eq(&b.column_4, epsilon, max_relative)
});

impl Matrix4A {
    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::wrap(glam::Mat4::IDENTITY)
    }

    /// Creates a matrix with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::wrap(glam::Mat4::ZERO)
    }

    /// Creates a diagonal matrix with the given vector as the diagonal.
    #[inline]
    pub const fn from_diagonal(diagonal: &Vector4A) -> Self {
        Self::wrap(glam::Mat4::from_diagonal(diagonal.unwrap()))
    }

    /// Creates a matrix with the given columns.
    #[inline]
    pub const fn from_columns(
        column_1: Vector4A,
        column_2: Vector4A,
        column_3: Vector4A,
        column_4: Vector4A,
    ) -> Self {
        Self::wrap(glam::Mat4::from_cols(
            column_1.unwrap(),
            column_2.unwrap(),
            column_3.unwrap(),
            column_4.unwrap(),
        ))
    }

    /// The first column of the matrix.
    #[inline]
    pub fn column_1(&self) -> &Vector4A {
        bytemuck::cast_ref(&self.inner.x_axis)
    }

    /// The second column of the matrix.
    #[inline]
    pub fn column_2(&self) -> &Vector4A {
        bytemuck::cast_ref(&self.inner.y_axis)
    }

    /// The third column of the matrix.
    #[inline]
    pub fn column_3(&self) -> &Vector4A {
        bytemuck::cast_ref(&self.inner.z_axis)
    }

    /// The fourth column of the matrix.
    #[inline]
    pub fn column_4(&self) -> &Vector4A {
        bytemuck::cast_ref(&self.inner.w_axis)
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub fn set_column_1(&mut self, column: Vector4A) {
        self.inner.x_axis = column.unwrap();
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub fn set_column_2(&mut self, column: Vector4A) {
        self.inner.y_axis = column.unwrap();
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub fn set_column_3(&mut self, column: Vector4A) {
        self.inner.z_axis = column.unwrap();
    }

    /// Sets the fourth column of the matrix to the given column.
    #[inline]
    pub fn set_column_4(&mut self, column: Vector4A) {
        self.inner.w_axis = column.unwrap();
    }

    /// Returns the element at row `i` and column `j`.
    ///
    /// # Panics
    /// If the indices are outside the matrix.
    #[inline]
    pub fn element(&self, i: usize, j: usize) -> f32 {
        let m = &self.inner;
        match j {
            0 => m.x_axis[i],
            1 => m.y_axis[i],
            2 => m.z_axis[i],
            3 => m.w_axis[i],
            _ => panic!("index out of bounds"),
        }
    }

    /// Returns a mutable reference to the element at row `i` and column `j`.
    ///
    /// # Panics
    /// If the indices are outside the matrix.
    #[inline]
    pub fn element_mut(&mut self, i: usize, j: usize) -> &mut f32 {
        let m = &mut self.inner;
        match j {
            0 => &mut m.x_axis[i],
            1 => &mut m.y_axis[i],
            2 => &mut m.z_axis[i],
            3 => &mut m.w_axis[i],
            _ => panic!("index out of bounds"),
        }
    }

    /// Returns the diagonal of this matrix as a vector.
    #[inline]
    pub fn diagonal(&self) -> Vector4A {
        let m = &self.inner;
        Vector4A::new(m.x_axis.x, m.y_axis.y, m.z_axis.z, m.w_axis.w)
    }

    /// Returns the inverse of this matrix. If the matrix is not invertible, the
    /// result will be non-finite.
    #[inline]
    pub fn inverted(&self) -> Self {
        Self::wrap(self.inner.inverse())
    }

    /// Returns the transpose of this matrix.
    #[inline]
    pub fn transposed(&self) -> Self {
        Self::wrap(self.inner.transpose())
    }

    /// Returns a matrix with the given closure applied to each element.
    #[inline]
    pub fn mapped(&self, mut f: impl FnMut(f32) -> f32) -> Self {
        let x = self.inner.x_axis;
        let y = self.inner.y_axis;
        let z = self.inner.z_axis;
        let w = self.inner.w_axis;
        Self::wrap(glam::Mat4::from_cols(
            glam::Vec4::new(f(x.x), f(x.y), f(x.z), f(x.w)),
            glam::Vec4::new(f(y.x), f(y.y), f(y.z), f(y.w)),
            glam::Vec4::new(f(z.x), f(z.y), f(z.z), f(z.w)),
            glam::Vec4::new(f(w.x), f(w.y), f(w.z), f(w.w)),
        ))
    }

    /// Returns the smallest element in the matrix.
    #[inline]
    pub fn min_element(&self) -> f32 {
        let m = &self.inner;
        m.x_axis
            .min_element()
            .min(m.y_axis.min_element())
            .min(m.z_axis.min_element())
            .min(m.w_axis.min_element())
    }

    /// Returns the largest element in the matrix.
    #[inline]
    pub fn max_element(&self) -> f32 {
        let m = &self.inner;
        m.x_axis
            .max_element()
            .max(m.y_axis.max_element())
            .max(m.z_axis.max_element())
            .max(m.w_axis.max_element())
    }

    /// Assuming this matrix represents a homogeneous transform, returns the
    /// upper left 3x3 matrix representing the linear (rotation and scaling)
    /// part of the transform.
    #[inline]
    pub fn linear_part(&self) -> Matrix3 {
        let m = &self.inner;
        Matrix3::from_glam(glam::Mat3::from_cols(
            m.x_axis.truncate(),
            m.y_axis.truncate(),
            m.z_axis.truncate(),
        ))
    }

    /// Assuming this matrix represents a homogeneous transform, incorporates
    /// the given translation to be applied after the transform.
    #[inline]
    pub fn translate_transform(&mut self, translation: &Vector3A) {
        let w = &mut self.inner.w_axis;
        *w += translation.extended(0.0).unwrap();
    }

    /// Assuming this matrix represents a homogeneous transform, incorporates
    /// the given rotation to be applied after the transform.
    #[inline]
    pub fn rotate_transform(&mut self, rotation: &UnitQuaternionA) {
        *self = rotation.to_homogeneous_matrix() * *self;
    }

    /// Assuming this matrix represents a homogeneous transform, incorporates
    /// the given scaling to be applied after the transform.
    #[inline]
    pub fn scale_transform(&mut self, scaling: f32) {
        self.inner.x_axis = (scaling * self.inner.x_axis.truncate()).extend(self.inner.x_axis.w);
        self.inner.y_axis = (scaling * self.inner.y_axis.truncate()).extend(self.inner.y_axis.w);
        self.inner.z_axis = (scaling * self.inner.z_axis.truncate()).extend(self.inner.z_axis.w);
        self.inner.w_axis = (scaling * self.inner.w_axis.truncate()).extend(self.inner.w_axis.w);
    }

    /// Assuming this matrix represents a homogeneous transform, applies the
    /// transform to the given point.
    #[inline]
    pub fn transform_point(&self, point: &Point3A) -> Point3A {
        Point3A::wrap(self.inner.transform_point3a(point.unwrap()))
    }

    /// Assuming this matrix represents a homogeneous transform, applies the
    /// transform to the given vector. The translation part of the transform is
    /// not applied to vectors.
    #[inline]
    pub fn transform_vector(&self, vector: &Vector3A) -> Vector3A {
        Vector3A::wrap(self.inner.transform_vector3a(vector.unwrap()))
    }

    /// Assuming this matrix represents a projection, projects the given point
    /// by applying the matrix and performing perspective division.
    #[inline]
    pub fn project_point(&self, point: &Point3A) -> Point3A {
        Point3A::wrap(self.inner.project_point3a(point.unwrap()))
    }

    /// Converts the matrix to the 4-byte aligned cache-friendly [`Matrix4`].
    #[inline]
    pub fn unaligned(&self) -> Matrix4 {
        Matrix4::from_columns(
            self.column_1().unaligned(),
            self.column_2().unaligned(),
            self.column_3().unaligned(),
            self.column_4().unaligned(),
        )
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Mat4) -> Self {
        Self { inner }
    }
}

impl_binop!(Add, add, Matrix4A, Matrix4A, Matrix4A, |a, b| {
    Matrix4A::wrap(a.inner.add_mat4(&b.inner))
});

impl_binop!(Sub, sub, Matrix4A, Matrix4A, Matrix4A, |a, b| {
    Matrix4A::wrap(a.inner.sub_mat4(&b.inner))
});

impl_binop!(Mul, mul, Matrix4A, Matrix4A, Matrix4A, |a, b| {
    Matrix4A::wrap(a.inner.mul_mat4(&b.inner))
});

impl_binop!(Mul, mul, Matrix4A, Vector4A, Vector4A, |a, b| {
    Vector4A::wrap(a.inner.mul_vec4(b.unwrap()))
});

impl_binop!(Mul, mul, Matrix4A, f32, Matrix4A, |a, b| {
    Matrix4A::wrap(a.inner.mul_scalar(*b))
});

impl_binop!(Mul, mul, f32, Matrix4A, Matrix4A, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Matrix4A, f32, Matrix4A, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, Matrix4A, Matrix4A, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Matrix4A, Matrix4A, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix4A, Matrix4A, |a, b| {
    a.inner.mul_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix4A, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Matrix4A, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Matrix4A, Matrix4A, |val| {
    Matrix4A::wrap(val.inner.neg())
});

impl_abs_diff_eq!(Matrix4A, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Matrix4A, |a, b, epsilon, max_relative| {
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
    use approx::assert_abs_diff_eq;

    // Test constants
    const EPSILON: f32 = 1e-6;

    // Matrix3 (unaligned) tests
    #[test]
    fn creating_matrix3_identity_gives_identity_matrix() {
        let identity = Matrix3::identity();

        // Check diagonal elements
        assert_eq!(identity.column_1().x(), 1.0);
        assert_eq!(identity.column_2().y(), 1.0);
        assert_eq!(identity.column_3().z(), 1.0);

        // Check off-diagonal elements are zero
        assert_eq!(identity.column_1().y(), 0.0);
        assert_eq!(identity.column_1().z(), 0.0);
        assert_eq!(identity.column_2().x(), 0.0);
        assert_eq!(identity.column_2().z(), 0.0);
        assert_eq!(identity.column_3().x(), 0.0);
        assert_eq!(identity.column_3().y(), 0.0);
    }

    #[test]
    fn creating_matrix3_zeros_gives_zero_matrix() {
        let zeros = Matrix3::zeros();

        assert_eq!(zeros.column_1().x(), 0.0);
        assert_eq!(zeros.column_1().y(), 0.0);
        assert_eq!(zeros.column_1().z(), 0.0);
        assert_eq!(zeros.column_2().x(), 0.0);
        assert_eq!(zeros.column_2().y(), 0.0);
        assert_eq!(zeros.column_2().z(), 0.0);
        assert_eq!(zeros.column_3().x(), 0.0);
        assert_eq!(zeros.column_3().y(), 0.0);
        assert_eq!(zeros.column_3().z(), 0.0);
    }

    #[test]
    fn creating_matrix3_from_diagonal_works() {
        let diag = Vector3::new(2.0, 3.0, 4.0);
        let matrix = Matrix3::from_diagonal(&diag);

        assert_eq!(matrix.column_1().x(), 2.0);
        assert_eq!(matrix.column_2().y(), 3.0);
        assert_eq!(matrix.column_3().z(), 4.0);

        // Check off-diagonal elements are zero
        assert_eq!(matrix.column_1().y(), 0.0);
        assert_eq!(matrix.column_1().z(), 0.0);
        assert_eq!(matrix.column_2().x(), 0.0);
        assert_eq!(matrix.column_2().z(), 0.0);
        assert_eq!(matrix.column_3().x(), 0.0);
        assert_eq!(matrix.column_3().y(), 0.0);
    }

    #[test]
    fn creating_matrix3_from_columns_works() {
        let col1 = Vector3::new(1.0, 2.0, 3.0);
        let col2 = Vector3::new(4.0, 5.0, 6.0);
        let col3 = Vector3::new(7.0, 8.0, 9.0);

        let matrix = Matrix3::from_columns(col1, col2, col3);

        assert_eq!(matrix.column_1(), &col1);
        assert_eq!(matrix.column_2(), &col2);
        assert_eq!(matrix.column_3(), &col3);
    }

    #[test]
    fn setting_matrix3_columns_works() {
        let mut matrix = Matrix3::zeros();
        let col1 = Vector3::new(1.0, 2.0, 3.0);
        let col2 = Vector3::new(4.0, 5.0, 6.0);
        let col3 = Vector3::new(7.0, 8.0, 9.0);

        matrix.set_column_1(col1);
        matrix.set_column_2(col2);
        matrix.set_column_3(col3);

        assert_eq!(matrix.column_1(), &col1);
        assert_eq!(matrix.column_2(), &col2);
        assert_eq!(matrix.column_3(), &col3);
    }

    #[test]
    fn converting_matrix3_to_aligned_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let aligned = matrix.aligned();

        assert_eq!(aligned.element(0, 0), 1.0);
        assert_eq!(aligned.element(1, 1), 2.0);
        assert_eq!(aligned.element(2, 2), 3.0);
    }

    #[test]
    fn converting_matrix3a_to_matrix3_works() {
        let matrix_a = Matrix3A::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let matrix = matrix_a.unaligned();

        assert_eq!(matrix.column_1().x(), 1.0);
        assert_eq!(matrix.column_2().y(), 2.0);
        assert_eq!(matrix.column_3().z(), 3.0);
    }

    // Matrix4 (unaligned) tests
    #[test]
    fn creating_matrix4_identity_gives_identity_matrix() {
        let identity = Matrix4::identity();

        // Check diagonal elements
        assert_eq!(identity.column_1().x(), 1.0);
        assert_eq!(identity.column_2().y(), 1.0);
        assert_eq!(identity.column_3().z(), 1.0);
        assert_eq!(identity.column_4().w(), 1.0);

        // Check off-diagonal elements in first column
        assert_eq!(identity.column_1().y(), 0.0);
        assert_eq!(identity.column_1().z(), 0.0);
        assert_eq!(identity.column_1().w(), 0.0);
        // Check off-diagonal elements in second column
        assert_eq!(identity.column_2().x(), 0.0);
        assert_eq!(identity.column_2().z(), 0.0);
        assert_eq!(identity.column_2().w(), 0.0);
    }

    #[test]
    fn creating_matrix4_zeros_gives_zero_matrix() {
        let zeros = Matrix4::zeros();

        // Check all elements are zero
        assert_eq!(zeros.column_1().x(), 0.0);
        assert_eq!(zeros.column_2().y(), 0.0);
        assert_eq!(zeros.column_3().z(), 0.0);
        assert_eq!(zeros.column_4().w(), 0.0);
        assert_eq!(zeros.column_1().w(), 0.0);
        assert_eq!(zeros.column_4().x(), 0.0);
    }

    #[test]
    fn creating_matrix4_from_diagonal_works() {
        let diag = Vector4::new(2.0, 3.0, 4.0, 5.0);
        let matrix = Matrix4::from_diagonal(&diag);

        assert_eq!(matrix.column_1().x(), 2.0);
        assert_eq!(matrix.column_2().y(), 3.0);
        assert_eq!(matrix.column_3().z(), 4.0);
        assert_eq!(matrix.column_4().w(), 5.0);

        // Check off-diagonal elements are zero
        assert_eq!(matrix.column_1().y(), 0.0);
        assert_eq!(matrix.column_1().z(), 0.0);
        assert_eq!(matrix.column_1().w(), 0.0);
        assert_eq!(matrix.column_2().x(), 0.0);
    }

    #[test]
    fn creating_matrix4_from_columns_works() {
        let col1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4::new(13.0, 14.0, 15.0, 16.0);

        let matrix = Matrix4::from_columns(col1, col2, col3, col4);

        assert_eq!(matrix.column_1(), &col1);
        assert_eq!(matrix.column_2(), &col2);
        assert_eq!(matrix.column_3(), &col3);
        assert_eq!(matrix.column_4(), &col4);
    }

    #[test]
    fn setting_matrix4_columns_works() {
        let mut matrix = Matrix4::zeros();
        let col1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4::new(13.0, 14.0, 15.0, 16.0);

        matrix.set_column_1(col1);
        matrix.set_column_2(col2);
        matrix.set_column_3(col3);
        matrix.set_column_4(col4);

        assert_eq!(matrix.column_1(), &col1);
        assert_eq!(matrix.column_2(), &col2);
        assert_eq!(matrix.column_3(), &col3);
        assert_eq!(matrix.column_4(), &col4);
    }

    #[test]
    fn converting_matrix4_to_aligned_works() {
        let matrix = Matrix4::from_diagonal(&Vector4::new(1.0, 2.0, 3.0, 4.0));
        let aligned = matrix.aligned();

        assert_eq!(aligned.element(0, 0), 1.0);
        assert_eq!(aligned.element(1, 1), 2.0);
        assert_eq!(aligned.element(2, 2), 3.0);
        assert_eq!(aligned.element(3, 3), 4.0);
    }

    #[test]
    fn converting_matrix4a_to_matrix4_works() {
        let matrix_a = Matrix4A::from_diagonal(&Vector4A::new(1.0, 2.0, 3.0, 4.0));
        let matrix = matrix_a.unaligned();

        assert_eq!(matrix.column_1().x(), 1.0);
        assert_eq!(matrix.column_2().y(), 2.0);
        assert_eq!(matrix.column_3().z(), 3.0);
        assert_eq!(matrix.column_4().w(), 4.0);
    }

    // Matrix3A (aligned) tests
    #[test]
    fn creating_matrix3a_identity_gives_identity_matrix() {
        let identity = Matrix3A::identity();
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
    fn creating_matrix3a_zeros_gives_zero_matrix() {
        let zeros = Matrix3A::zeros();
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(zeros.element(i, j), 0.0);
            }
        }
    }

    #[test]
    fn creating_matrix3a_from_diagonal_works() {
        let diag = Vector3::new(2.0, 3.0, 4.0);
        let matrix = Matrix3A::from_diagonal(&diag);

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
    fn creating_matrix3a_from_columns_works() {
        let col1 = Vector3A::new(1.0, 2.0, 3.0);
        let col2 = Vector3A::new(4.0, 5.0, 6.0);
        let col3 = Vector3A::new(7.0, 8.0, 9.0);

        let matrix = Matrix3A::from_columns(col1, col2, col3);

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
    fn accessing_matrix3a_elements_works() {
        let mut matrix = Matrix3A::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));

        assert_eq!(matrix.element(0, 0), 1.0);
        assert_eq!(matrix.element(1, 1), 2.0);
        assert_eq!(matrix.element(2, 2), 3.0);

        *matrix.element_mut(0, 1) = 5.0;
        assert_eq!(matrix.element(0, 1), 5.0);
    }

    #[test]
    fn accessing_matrix3a_columns_works() {
        let col1 = Vector3A::new(1.0, 2.0, 3.0);
        let col2 = Vector3A::new(4.0, 5.0, 6.0);
        let col3 = Vector3A::new(7.0, 8.0, 9.0);
        let matrix = Matrix3A::from_columns(col1, col2, col3);

        let extracted_col1 = matrix.column_1();
        let extracted_col2 = matrix.column_2();
        let extracted_col3 = matrix.column_3();

        assert_eq!(extracted_col1, &col1);
        assert_eq!(extracted_col2, &col2);
        assert_eq!(extracted_col3, &col3);
    }

    #[test]
    fn setting_matrix3a_columns_works() {
        let mut matrix = Matrix3A::zeros();
        let col1 = Vector3A::new(1.0, 2.0, 3.0);
        let col2 = Vector3A::new(4.0, 5.0, 6.0);
        let col3 = Vector3A::new(7.0, 8.0, 9.0);

        matrix.set_column_1(col1);
        matrix.set_column_2(col2);
        matrix.set_column_3(col3);

        assert_eq!(matrix.column_1(), &col1);
        assert_eq!(matrix.column_2(), &col2);
        assert_eq!(matrix.column_3(), &col3);
    }

    #[test]
    fn extracting_matrix3a_diagonal_works() {
        let diag_vec = Vector3::new(2.0, 3.0, 4.0);
        let matrix = Matrix3A::from_diagonal(&diag_vec);
        let extracted_diag = matrix.diagonal();

        assert_eq!(extracted_diag.unaligned(), diag_vec);
    }

    #[test]
    fn transposing_matrix3a_works() {
        let col1 = Vector3A::new(1.0, 2.0, 3.0);
        let col2 = Vector3A::new(4.0, 5.0, 6.0);
        let col3 = Vector3A::new(7.0, 8.0, 9.0);
        let matrix = Matrix3A::from_columns(col1, col2, col3);

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
    fn negating_matrix3a_works() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(2.0, -3.0, 4.0));
        let negated = -matrix;

        assert_eq!(negated.element(0, 0), -2.0);
        assert_eq!(negated.element(1, 1), 3.0);
        assert_eq!(negated.element(2, 2), -4.0);
    }

    #[test]
    fn mapping_matrix3a_elements_works() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let mapped = matrix.mapped(|x| x * 2.0);

        assert_eq!(mapped.element(0, 0), 2.0);
        assert_eq!(mapped.element(1, 1), 4.0);
        assert_eq!(mapped.element(2, 2), 6.0);
    }

    #[test]
    fn inverting_matrix3a_works() {
        let identity = Matrix3A::identity();
        let inverted = identity.inverted();

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
    fn finding_matrix3a_min_element_works() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(5.0, 1.0, 3.0));
        assert_abs_diff_eq!(matrix.min_element(), 0.0, epsilon = EPSILON); // off-diagonal zeros
    }

    #[test]
    fn finding_matrix3a_max_element_works() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(1.0, 5.0, 3.0));
        assert_abs_diff_eq!(matrix.max_element(), 5.0, epsilon = EPSILON);
    }

    #[test]
    fn converting_matrix3a_to_unaligned_works() {
        let matrix_a = Matrix3A::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let matrix = matrix_a.unaligned();

        assert_eq!(matrix.column_1().x(), 1.0);
        assert_eq!(matrix.column_2().y(), 2.0);
        assert_eq!(matrix.column_3().z(), 3.0);
    }

    #[test]
    fn converting_matrix3_to_matrix3a_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let matrix_a = matrix.aligned();

        assert_eq!(matrix_a.element(0, 0), 1.0);
        assert_eq!(matrix_a.element(1, 1), 2.0);
        assert_eq!(matrix_a.element(2, 2), 3.0);
    }

    #[test]
    fn matrix3a_arithmetic_operations_work() {
        let m1 = Matrix3A::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let m2 = Matrix3A::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));

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
    fn matrix3a_vector_multiplication_works() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));
        let vector = Vector3A::new(1.0, 1.0, 1.0);

        let result = &matrix * &vector;
        assert_eq!(result.x(), 2.0);
        assert_eq!(result.y(), 3.0);
        assert_eq!(result.z(), 4.0);
    }

    #[test]
    fn matrix3a_scalar_multiplication_works() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));

        let mul_right = &matrix * 2.0;
        assert_eq!(mul_right.element(0, 0), 2.0);
        assert_eq!(mul_right.element(1, 1), 4.0);
        assert_eq!(mul_right.element(2, 2), 6.0);

        let mul_left = 3.0 * &matrix;
        assert_eq!(mul_left.element(0, 0), 3.0);
        assert_eq!(mul_left.element(1, 1), 6.0);
        assert_eq!(mul_left.element(2, 2), 9.0);
    }

    #[test]
    fn matrix3a_division_works() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(2.0, 4.0, 6.0));
        let divided = &matrix / 2.0;

        assert_eq!(divided.element(0, 0), 1.0);
        assert_eq!(divided.element(1, 1), 2.0);
        assert_eq!(divided.element(2, 2), 3.0);
    }

    #[test]
    fn matrix3a_assignment_operations_work() {
        let mut matrix1 = Matrix3A::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let matrix2 = Matrix3A::from_diagonal(&Vector3::new(1.0, 1.0, 1.0));

        matrix1 += matrix2;
        assert_eq!(matrix1.element(0, 0), 2.0);
        assert_eq!(matrix1.element(1, 1), 3.0);
        assert_eq!(matrix1.element(2, 2), 4.0);

        matrix1 -= matrix2;
        assert_eq!(matrix1.element(0, 0), 1.0);
        assert_eq!(matrix1.element(1, 1), 2.0);
        assert_eq!(matrix1.element(2, 2), 3.0);

        matrix1 *= 2.0;
        assert_eq!(matrix1.element(0, 0), 2.0);
        assert_eq!(matrix1.element(1, 1), 4.0);
        assert_eq!(matrix1.element(2, 2), 6.0);

        matrix1 /= 2.0;
        assert_eq!(matrix1.element(0, 0), 1.0);
        assert_eq!(matrix1.element(1, 1), 2.0);
        assert_eq!(matrix1.element(2, 2), 3.0);
    }

    // Matrix4A (aligned) tests
    #[test]
    fn creating_matrix4a_identity_gives_identity_matrix() {
        let identity = Matrix4A::identity();
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
    fn creating_matrix4a_zeros_gives_zero_matrix() {
        let zeros = Matrix4A::zeros();
        for i in 0..4 {
            for j in 0..4 {
                assert_eq!(zeros.element(i, j), 0.0);
            }
        }
    }

    #[test]
    fn creating_matrix4a_from_diagonal_works() {
        let diag = Vector4A::new(2.0, 3.0, 4.0, 5.0);
        let matrix = Matrix4A::from_diagonal(&diag);

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
    fn creating_matrix4a_from_columns_works() {
        let col1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4A::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4A::new(13.0, 14.0, 15.0, 16.0);

        let matrix = Matrix4A::from_columns(col1, col2, col3, col4);

        for i in 0..4 {
            for j in 0..4 {
                let expected = (j * 4 + i + 1) as f32;
                assert_eq!(matrix.element(i, j), expected);
            }
        }
    }

    #[test]
    fn accessing_matrix4a_elements_works() {
        let mut matrix = Matrix4A::from_diagonal(&Vector4A::new(1.0, 2.0, 3.0, 4.0));

        assert_eq!(matrix.element(0, 0), 1.0);
        assert_eq!(matrix.element(1, 1), 2.0);
        assert_eq!(matrix.element(2, 2), 3.0);
        assert_eq!(matrix.element(3, 3), 4.0);

        *matrix.element_mut(0, 1) = 5.0;
        assert_eq!(matrix.element(0, 1), 5.0);
    }

    #[test]
    fn accessing_matrix4a_columns_works() {
        let col1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4A::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4A::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4A::from_columns(col1, col2, col3, col4);

        let extracted_col1 = matrix.column_1();
        let extracted_col2 = matrix.column_2();
        let extracted_col3 = matrix.column_3();
        let extracted_col4 = matrix.column_4();

        assert_eq!(extracted_col1, &col1);
        assert_eq!(extracted_col2, &col2);
        assert_eq!(extracted_col3, &col3);
        assert_eq!(extracted_col4, &col4);
    }

    #[test]
    fn setting_matrix4a_columns_works() {
        let mut matrix = Matrix4A::zeros();
        let col1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4A::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4A::new(13.0, 14.0, 15.0, 16.0);

        matrix.set_column_1(col1);
        matrix.set_column_2(col2);
        matrix.set_column_3(col3);
        matrix.set_column_4(col4);

        assert_eq!(matrix.column_1(), &col1);
        assert_eq!(matrix.column_2(), &col2);
        assert_eq!(matrix.column_3(), &col3);
        assert_eq!(matrix.column_4(), &col4);
    }

    #[test]
    fn extracting_matrix4a_diagonal_works() {
        let diag_vec = Vector4A::new(2.0, 3.0, 4.0, 5.0);
        let matrix = Matrix4A::from_diagonal(&diag_vec);
        let extracted_diag = matrix.diagonal();

        assert_eq!(extracted_diag, diag_vec);
    }

    #[test]
    fn extracting_matrix4a_linear_part_works() {
        let col1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4A::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4A::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4A::from_columns(col1, col2, col3, col4);

        let linear = matrix.linear_part().aligned();

        // Linear part is the upper-left 3x3 submatrix
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(linear.element(i, j), matrix.element(i, j));
            }
        }
    }

    #[test]
    fn transposing_matrix4a_works() {
        let col1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4A::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4A::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4A::from_columns(col1, col2, col3, col4);

        let transposed = matrix.transposed();

        // Original columns become rows in transposed
        assert_eq!(transposed.element(0, 0), 1.0);
        assert_eq!(transposed.element(0, 1), 2.0);
        assert_eq!(transposed.element(0, 2), 3.0);
        assert_eq!(transposed.element(0, 3), 4.0);
    }

    #[test]
    fn negating_matrix4a_works() {
        let matrix = Matrix4A::from_diagonal(&Vector4A::new(2.0, -3.0, 4.0, -5.0));
        let negated = -matrix;

        assert_eq!(negated.element(0, 0), -2.0);
        assert_eq!(negated.element(1, 1), 3.0);
        assert_eq!(negated.element(2, 2), -4.0);
        assert_eq!(negated.element(3, 3), 5.0);
    }

    #[test]
    fn mapping_matrix4a_elements_works() {
        let matrix = Matrix4A::from_diagonal(&Vector4A::new(1.0, 2.0, 3.0, 4.0));
        let mapped = matrix.mapped(|x| x * 2.0);

        assert_eq!(mapped.element(0, 0), 2.0);
        assert_eq!(mapped.element(1, 1), 4.0);
        assert_eq!(mapped.element(2, 2), 6.0);
        assert_eq!(mapped.element(3, 3), 8.0);
    }

    #[test]
    fn inverting_matrix4a_works() {
        let identity = Matrix4A::identity();
        let inverted = identity.inverted();

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
    fn finding_matrix4a_min_element_works() {
        let matrix = Matrix4A::from_diagonal(&Vector4A::new(7.0, 1.0, 3.0, 2.0));
        assert_abs_diff_eq!(matrix.min_element(), 0.0, epsilon = EPSILON); // off-diagonal zeros
    }

    #[test]
    fn finding_matrix4a_max_element_works() {
        let matrix = Matrix4A::from_diagonal(&Vector4A::new(1.0, 7.0, 3.0, 2.0));
        assert_abs_diff_eq!(matrix.max_element(), 7.0, epsilon = EPSILON);
    }

    #[test]
    fn applying_matrix4a_translate_transform_works() {
        let mut matrix = Matrix4A::identity();
        let translation = Vector3A::new(1.0, 2.0, 3.0);

        matrix.translate_transform(&translation);

        // Translation should be in the last column
        assert_eq!(matrix.element(0, 3), 1.0);
        assert_eq!(matrix.element(1, 3), 2.0);
        assert_eq!(matrix.element(2, 3), 3.0);
    }

    #[test]
    fn applying_matrix4a_scale_transform_works() {
        let mut matrix = Matrix4A::identity();
        matrix.set_column_4(Vector4A::same(1.0));
        matrix.scale_transform(2.0);

        // Scaling affects the diagonal
        assert_eq!(matrix.element(0, 0), 2.0);
        assert_eq!(matrix.element(1, 1), 2.0);
        assert_eq!(matrix.element(2, 2), 2.0);

        // Scaling affects the translation
        assert_eq!(matrix.element(0, 3), 2.0);
        assert_eq!(matrix.element(1, 3), 2.0);
        assert_eq!(matrix.element(2, 3), 2.0);
        assert_eq!(matrix.element(3, 3), 1.0);
    }

    #[test]
    fn transforming_point_with_matrix4a_works() {
        let mut matrix = Matrix4A::identity();
        let translation = Vector3A::new(1.0, 2.0, 3.0);
        matrix.translate_transform(&translation);

        let point = Point3A::new(0.0, 0.0, 0.0);
        let transformed = matrix.transform_point(&point);

        assert_eq!(transformed.x(), 1.0);
        assert_eq!(transformed.y(), 2.0);
        assert_eq!(transformed.z(), 3.0);
    }

    #[test]
    fn transforming_vector_with_matrix4a_works() {
        let mut matrix = Matrix4A::identity();
        matrix.scale_transform(2.0);

        let vector = Vector3A::new(1.0, 2.0, 3.0);
        let transformed = matrix.transform_vector(&vector);

        assert_eq!(transformed.x(), 2.0);
        assert_eq!(transformed.y(), 4.0);
        assert_eq!(transformed.z(), 6.0);
    }

    #[test]
    fn projecting_point_with_matrix4a_works() {
        let matrix = Matrix4A::identity();
        let point = Point3A::new(1.0, 2.0, 3.0);
        let projected = matrix.project_point(&point);

        // Identity projection should leave point unchanged
        assert_eq!(projected.x(), 1.0);
        assert_eq!(projected.y(), 2.0);
        assert_eq!(projected.z(), 3.0);
    }

    #[test]
    fn converting_matrix4a_to_unaligned_works() {
        let matrix_a = Matrix4A::from_diagonal(&Vector4A::new(1.0, 2.0, 3.0, 4.0));
        let matrix = matrix_a.unaligned();

        assert_eq!(matrix.column_1().x(), 1.0);
        assert_eq!(matrix.column_2().y(), 2.0);
        assert_eq!(matrix.column_3().z(), 3.0);
        assert_eq!(matrix.column_4().w(), 4.0);
    }

    #[test]
    fn converting_matrix4_to_matrix4a_works() {
        let matrix = Matrix4::from_diagonal(&Vector4::new(1.0, 2.0, 3.0, 4.0));
        let matrix_a = matrix.aligned();

        assert_eq!(matrix_a.element(0, 0), 1.0);
        assert_eq!(matrix_a.element(1, 1), 2.0);
        assert_eq!(matrix_a.element(2, 2), 3.0);
        assert_eq!(matrix_a.element(3, 3), 4.0);
    }

    #[test]
    fn matrix4a_arithmetic_operations_work() {
        let m1 = Matrix4A::from_diagonal(&Vector4A::new(1.0, 2.0, 3.0, 4.0));
        let m2 = Matrix4A::from_diagonal(&Vector4A::new(2.0, 3.0, 4.0, 5.0));

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
    fn matrix4a_vector_multiplication_works() {
        let matrix = Matrix4A::from_diagonal(&Vector4A::new(2.0, 3.0, 4.0, 5.0));
        let vector = Vector4A::new(1.0, 1.0, 1.0, 1.0);

        let result = &matrix * &vector;
        assert_eq!(result.x(), 2.0);
        assert_eq!(result.y(), 3.0);
        assert_eq!(result.z(), 4.0);
        assert_eq!(result.w(), 5.0);
    }

    #[test]
    fn matrix4a_scalar_multiplication_works() {
        let matrix = Matrix4A::from_diagonal(&Vector4A::new(1.0, 2.0, 3.0, 4.0));

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
    fn matrix4a_division_works() {
        let matrix = Matrix4A::from_diagonal(&Vector4A::new(2.0, 4.0, 6.0, 8.0));
        let divided = &matrix / 2.0;

        assert_eq!(divided.element(0, 0), 1.0);
        assert_eq!(divided.element(1, 1), 2.0);
        assert_eq!(divided.element(2, 2), 3.0);
        assert_eq!(divided.element(3, 3), 4.0);
    }

    #[test]
    fn matrix4a_assignment_operations_work() {
        let mut matrix1 = Matrix4A::from_diagonal(&Vector4A::new(1.0, 2.0, 3.0, 4.0));
        let matrix2 = Matrix4A::from_diagonal(&Vector4A::new(1.0, 1.0, 1.0, 1.0));

        matrix1 += matrix2;
        assert_eq!(matrix1.element(0, 0), 2.0);
        assert_eq!(matrix1.element(1, 1), 3.0);
        assert_eq!(matrix1.element(2, 2), 4.0);
        assert_eq!(matrix1.element(3, 3), 5.0);

        matrix1 -= matrix2;
        assert_eq!(matrix1.element(0, 0), 1.0);
        assert_eq!(matrix1.element(1, 1), 2.0);
        assert_eq!(matrix1.element(2, 2), 3.0);
        assert_eq!(matrix1.element(3, 3), 4.0);

        matrix1 *= 2.0;
        assert_eq!(matrix1.element(0, 0), 2.0);
        assert_eq!(matrix1.element(1, 1), 4.0);
        assert_eq!(matrix1.element(2, 2), 6.0);
        assert_eq!(matrix1.element(3, 3), 8.0);

        matrix1 /= 2.0;
        assert_eq!(matrix1.element(0, 0), 1.0);
        assert_eq!(matrix1.element(1, 1), 2.0);
        assert_eq!(matrix1.element(2, 2), 3.0);
        assert_eq!(matrix1.element(3, 3), 4.0);
    }

    // General matrix property tests
    #[test]
    fn matrix_operations_with_different_reference_combinations_work() {
        let m1 = Matrix3A::identity();
        let m2 = Matrix3A::identity();

        // Test all combinations of reference/owned for binary operations
        let _result1 = &m1 + &m2; // ref + ref
        let _result2 = &m1 + m2; // ref + owned
        let _result3 = m1 + &m2; // owned + ref
        let _result4 = m1 + m2; // owned + owned

        // Recreate since they were moved
        let m1 = Matrix3A::identity();
        let _result5 = 2.0 * &m1; // scalar * ref
        let _result6 = 2.0 * m1; // scalar * owned

        let m1 = Matrix3A::identity();
        let _result7 = &m1 * 2.0; // ref * scalar
        let _result8 = m1 * 2.0; // owned * scalar
    }

    #[test]
    fn matrix_arithmetic_maintains_precision() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(0.1, 0.2, 0.3));
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
        let identity3 = Matrix3A::identity();
        let test_matrix3 = Matrix3A::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));

        let left_mult = &identity3 * &test_matrix3;
        let right_mult = &test_matrix3 * &identity3;

        // I * M = M * I = M
        assert_eq!(left_mult, test_matrix3);
        assert_eq!(right_mult, test_matrix3);

        // Same for Matrix4A
        let identity4 = Matrix4A::identity();
        let test_matrix4 = Matrix4A::from_diagonal(&Vector4A::new(2.0, 3.0, 4.0, 5.0));

        let left_mult4 = &identity4 * &test_matrix4;
        let right_mult4 = &test_matrix4 * &identity4;

        assert_eq!(left_mult4, test_matrix4);
        assert_eq!(right_mult4, test_matrix4);
    }

    #[test]
    fn matrix_transpose_is_involutive() {
        let matrix = Matrix3A::from_columns(
            Vector3A::new(1.0, 2.0, 3.0),
            Vector3A::new(4.0, 5.0, 6.0),
            Vector3A::new(7.0, 8.0, 9.0),
        );

        let double_transposed = matrix.transposed().transposed();

        // (A^T)^T = A
        assert_eq!(double_transposed, matrix);
    }

    #[test]
    fn matrix_inversion_properties_hold() {
        let matrix = Matrix3A::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));
        let inverse = matrix.inverted();
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
        let mut matrix = Matrix4A::identity();
        let translation = Vector3A::new(1.0, 2.0, 3.0);
        let scale = 2.0;

        matrix.translate_transform(&translation);
        matrix.scale_transform(scale);

        let point = Point3A::new(1.0, 1.0, 1.0);
        let transformed = matrix.transform_point(&point);

        // Transform order is translate first, then scale: ((1,1,1) + (1,2,3)) * 2 = (4,6,8)
        assert_abs_diff_eq!(transformed.x(), 4.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 6.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 8.0, epsilon = EPSILON);
    }

    #[test]
    fn matrix_vector_vs_point_transform_difference() {
        let mut matrix = Matrix4A::identity();
        let translation = Vector3A::new(5.0, 5.0, 5.0);
        matrix.translate_transform(&translation);

        let vector = Vector3A::new(1.0, 1.0, 1.0);
        let point = Point3A::new(1.0, 1.0, 1.0);

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
