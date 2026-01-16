//! Matrices.

use crate::{
    point::Point3,
    quaternion::UnitQuaternion,
    vector::{Vector3, Vector3C, Vector4, Vector4C},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;
use std::{fmt, ops::Mul};

/// A 3x3 matrix.
///
/// The columns are stored in 128-bit SIMD registers for efficient computation.
/// That leads to an extra 12 bytes in size (4 per column) and 16-byte
/// alignment. For cache-friendly storage, prefer the compact 4-byte aligned
/// [`Matrix3C`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix3 {
    inner: glam::Mat3A,
}

/// A 3x3 matrix. This is the "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Matrix3`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 9]", from = "[f32; 9]")
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix3C {
    column_1: Vector3C,
    column_2: Vector3C,
    column_3: Vector3C,
}

/// A 4x4 matrix.
///
/// The columns are stored in 128-bit SIMD registers for efficient computation.
/// That leads to an alignment of 16 bytes. For padding-free storage together
/// with smaller types, prefer the 4-byte aligned [`Matrix4C`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix4 {
    inner: glam::Mat4,
}

/// A 4x4 vector. This is the "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`Matrix4`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 16]", from = "[f32; 16]")
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Matrix4C {
    column_1: Vector4C,
    column_2: Vector4C,
    column_3: Vector4C,
    column_4: Vector4C,
}

impl Matrix3 {
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
    pub fn from_diagonal(diagonal: &Vector3) -> Self {
        Self::wrap(glam::Mat3A::from_diagonal(diagonal.unwrap().to_vec3()))
    }

    /// Creates a matrix with the given columns.
    #[inline]
    pub const fn from_columns(column_1: Vector3, column_2: Vector3, column_3: Vector3) -> Self {
        Self::wrap(glam::Mat3A::from_cols(
            column_1.unwrap(),
            column_2.unwrap(),
            column_3.unwrap(),
        ))
    }

    /// The first column of the matrix.
    #[inline]
    pub fn column_1(&self) -> &Vector3 {
        bytemuck::cast_ref(&self.inner.x_axis)
    }

    /// The second column of the matrix.
    #[inline]
    pub fn column_2(&self) -> &Vector3 {
        bytemuck::cast_ref(&self.inner.y_axis)
    }

    /// The third column of the matrix.
    #[inline]
    pub fn column_3(&self) -> &Vector3 {
        bytemuck::cast_ref(&self.inner.z_axis)
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub fn set_column_1(&mut self, column: Vector3) {
        self.inner.x_axis = column.unwrap();
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub fn set_column_2(&mut self, column: Vector3) {
        self.inner.y_axis = column.unwrap();
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub fn set_column_3(&mut self, column: Vector3) {
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
    pub fn inverse(&self) -> Self {
        Self::wrap(self.inner.inverse())
    }

    /// Returns the transpose of this matrix.
    #[inline]
    pub fn transpose(&self) -> Self {
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
    pub fn diagonal(&self) -> Vector3 {
        let m = &self.inner;
        Vector3::new(m.x_axis.x, m.y_axis.y, m.z_axis.z)
    }

    /// Returns the determinant of the matrix.
    #[inline]
    pub fn determinant(&self) -> f32 {
        self.inner.determinant()
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

    /// Converts the matrix to the 4-byte aligned cache-friendly [`Matrix3C`].
    #[inline]
    pub fn compact(&self) -> Matrix3C {
        Matrix3C::from_columns(
            self.column_1().compact(),
            self.column_2().compact(),
            self.column_3().compact(),
        )
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Mat3A) -> Self {
        Self { inner }
    }
}

impl_binop!(Add, add, Matrix3, Matrix3, Matrix3, |a, b| {
    Matrix3::wrap(a.inner.add_mat3(&b.inner))
});

impl_binop!(Sub, sub, Matrix3, Matrix3, Matrix3, |a, b| {
    Matrix3::wrap(a.inner.sub_mat3(&b.inner))
});

impl_binop!(Mul, mul, Matrix3, Matrix3, Matrix3, |a, b| {
    Matrix3::wrap(a.inner.mul_mat3(&b.inner))
});

impl_binop!(Mul, mul, Matrix3, Vector3, Vector3, |a, b| {
    Vector3::wrap(a.inner.mul_vec3a(b.unwrap()))
});

impl_binop!(Mul, mul, Matrix3, f32, Matrix3, |a, b| {
    Matrix3::wrap(a.inner.mul_scalar(*b))
});

impl_binop!(Mul, mul, f32, Matrix3, Matrix3, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Matrix3, f32, Matrix3, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Matrix3, Matrix3, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Matrix3, Matrix3, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix3, Matrix3, |a, b| {
    a.inner.mul_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix3, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Matrix3, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Matrix3, Matrix3, |val| {
    Matrix3::wrap(val.inner.neg())
});

impl_abs_diff_eq!(Matrix3, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Matrix3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Matrix3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let col1 = self.inner.x_axis;
        let col2 = self.inner.y_axis;
        let col3 = self.inner.z_axis;
        f.debug_struct("Matrix3")
            .field("column_1", &[col1.x, col1.y, col1.z])
            .field("column_2", &[col2.x, col2.y, col2.z])
            .field("column_3", &[col3.x, col3.y, col3.z])
            .finish()
    }
}

impl Matrix3C {
    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_columns(Vector3C::unit_x(), Vector3C::unit_y(), Vector3C::unit_z())
    }

    /// Creates a matrix with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::from_columns(Vector3C::zeros(), Vector3C::zeros(), Vector3C::zeros())
    }

    /// Creates a diagonal matrix with the given vector as the diagonal.
    #[inline]
    pub const fn from_diagonal(diagonal: &Vector3C) -> Self {
        let mut m = Self::zeros();
        *m.column_1.x_mut() = diagonal.x();
        *m.column_2.y_mut() = diagonal.y();
        *m.column_3.z_mut() = diagonal.z();
        m
    }

    /// Creates a matrix with the given columns.
    #[inline]
    pub const fn from_columns(column_1: Vector3C, column_2: Vector3C, column_3: Vector3C) -> Self {
        Self {
            column_1,
            column_2,
            column_3,
        }
    }

    /// The first column of the matrix.
    #[inline]
    pub const fn column_1(&self) -> &Vector3C {
        &self.column_1
    }

    /// The second column of the matrix.
    #[inline]
    pub const fn column_2(&self) -> &Vector3C {
        &self.column_2
    }

    /// The third column of the matrix.
    #[inline]
    pub const fn column_3(&self) -> &Vector3C {
        &self.column_3
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub const fn set_column_1(&mut self, column: Vector3C) {
        self.column_1 = column;
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub const fn set_column_2(&mut self, column: Vector3C) {
        self.column_2 = column;
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub const fn set_column_3(&mut self, column: Vector3C) {
        self.column_3 = column;
    }

    /// Converts the matrix to the 16-byte aligned SIMD-friendly [`Matrix3`].
    #[inline]
    pub fn aligned(&self) -> Matrix3 {
        Matrix3::from_columns(
            self.column_1().aligned(),
            self.column_2().aligned(),
            self.column_3().aligned(),
        )
    }
}

impl From<Matrix3C> for [f32; 9] {
    fn from(m: Matrix3C) -> [f32; 9] {
        [
            m.column_1.x(),
            m.column_1.y(),
            m.column_1.z(),
            m.column_2.x(),
            m.column_2.y(),
            m.column_2.z(),
            m.column_3.x(),
            m.column_3.y(),
            m.column_3.z(),
        ]
    }
}

impl From<[f32; 9]> for Matrix3C {
    fn from(arr: [f32; 9]) -> Matrix3C {
        Matrix3C::from_columns(
            Vector3C::new(arr[0], arr[1], arr[2]),
            Vector3C::new(arr[3], arr[4], arr[5]),
            Vector3C::new(arr[6], arr[7], arr[8]),
        )
    }
}

impl_abs_diff_eq!(Matrix3C, |a, b, epsilon| {
    a.column_1.abs_diff_eq(&b.column_1, epsilon)
        && a.column_2.abs_diff_eq(&b.column_2, epsilon)
        && a.column_3.abs_diff_eq(&b.column_3, epsilon)
});

impl_relative_eq!(Matrix3C, |a, b, epsilon, max_relative| {
    a.column_1.relative_eq(&b.column_1, epsilon, max_relative)
        && a.column_2.relative_eq(&b.column_2, epsilon, max_relative)
        && a.column_3.relative_eq(&b.column_3, epsilon, max_relative)
});

impl Matrix4 {
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
    pub const fn from_diagonal(diagonal: &Vector4) -> Self {
        Self::wrap(glam::Mat4::from_diagonal(diagonal.unwrap()))
    }

    /// Creates a matrix with the given columns.
    #[inline]
    pub const fn from_columns(
        column_1: Vector4,
        column_2: Vector4,
        column_3: Vector4,
        column_4: Vector4,
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
    pub fn column_1(&self) -> &Vector4 {
        bytemuck::cast_ref(&self.inner.x_axis)
    }

    /// The second column of the matrix.
    #[inline]
    pub fn column_2(&self) -> &Vector4 {
        bytemuck::cast_ref(&self.inner.y_axis)
    }

    /// The third column of the matrix.
    #[inline]
    pub fn column_3(&self) -> &Vector4 {
        bytemuck::cast_ref(&self.inner.z_axis)
    }

    /// The fourth column of the matrix.
    #[inline]
    pub fn column_4(&self) -> &Vector4 {
        bytemuck::cast_ref(&self.inner.w_axis)
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub fn set_column_1(&mut self, column: Vector4) {
        self.inner.x_axis = column.unwrap();
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub fn set_column_2(&mut self, column: Vector4) {
        self.inner.y_axis = column.unwrap();
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub fn set_column_3(&mut self, column: Vector4) {
        self.inner.z_axis = column.unwrap();
    }

    /// Sets the fourth column of the matrix to the given column.
    #[inline]
    pub fn set_column_4(&mut self, column: Vector4) {
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
    pub fn diagonal(&self) -> Vector4 {
        let m = &self.inner;
        Vector4::new(m.x_axis.x, m.y_axis.y, m.z_axis.z, m.w_axis.w)
    }

    /// Returns the inverse of this matrix. If the matrix is not invertible, the
    /// result will be non-finite.
    #[inline]
    pub fn inverse(&self) -> Self {
        Self::wrap(self.inner.inverse())
    }

    /// Returns the transpose of this matrix.
    #[inline]
    pub fn transpose(&self) -> Self {
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
        Matrix3::wrap(glam::Mat3A::from_cols(
            m.x_axis.truncate().to_vec3a(),
            m.y_axis.truncate().to_vec3a(),
            m.z_axis.truncate().to_vec3a(),
        ))
    }

    /// Assuming this matrix represents a homogeneous transform, incorporates
    /// the given translation to be applied after the transform.
    #[inline]
    pub fn translate_transform(&mut self, translation: &Vector3) {
        let w = &mut self.inner.w_axis;
        *w += translation.extended(0.0).unwrap();
    }

    /// Assuming this matrix represents a homogeneous transform, incorporates
    /// the given rotation to be applied after the transform.
    #[inline]
    pub fn rotate_transform(&mut self, rotation: &UnitQuaternion) {
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
    pub fn transform_point(&self, point: &Point3) -> Point3 {
        Point3::wrap(self.inner.transform_point3a(point.unwrap()))
    }

    /// Assuming this matrix represents a homogeneous transform, applies the
    /// transform to the given vector. The translation part of the transform is
    /// not applied to vectors.
    #[inline]
    pub fn transform_vector(&self, vector: &Vector3) -> Vector3 {
        Vector3::wrap(self.inner.transform_vector3a(vector.unwrap()))
    }

    /// Assuming this matrix represents a projection, projects the given point
    /// by applying the matrix and performing perspective division.
    #[inline]
    pub fn project_point(&self, point: &Point3) -> Point3 {
        Point3::wrap(self.inner.project_point3a(point.unwrap()))
    }

    /// Converts the matrix to the 4-byte aligned cache-friendly [`Matrix4C`].
    #[inline]
    pub fn compact(&self) -> Matrix4C {
        Matrix4C::from_columns(
            self.column_1().compact(),
            self.column_2().compact(),
            self.column_3().compact(),
            self.column_4().compact(),
        )
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Mat4) -> Self {
        Self { inner }
    }
}

impl_binop!(Add, add, Matrix4, Matrix4, Matrix4, |a, b| {
    Matrix4::wrap(a.inner.add_mat4(&b.inner))
});

impl_binop!(Sub, sub, Matrix4, Matrix4, Matrix4, |a, b| {
    Matrix4::wrap(a.inner.sub_mat4(&b.inner))
});

impl_binop!(Mul, mul, Matrix4, Matrix4, Matrix4, |a, b| {
    Matrix4::wrap(a.inner.mul_mat4(&b.inner))
});

impl_binop!(Mul, mul, Matrix4, Vector4, Vector4, |a, b| {
    Vector4::wrap(a.inner.mul_vec4(b.unwrap()))
});

impl_binop!(Mul, mul, Matrix4, f32, Matrix4, |a, b| {
    Matrix4::wrap(a.inner.mul_scalar(*b))
});

impl_binop!(Mul, mul, f32, Matrix4, Matrix4, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Matrix4, f32, Matrix4, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Matrix4, Matrix4, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Matrix4, Matrix4, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix4, Matrix4, |a, b| {
    a.inner.mul_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Matrix4, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Matrix4, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Matrix4, Matrix4, |val| {
    Matrix4::wrap(val.inner.neg())
});

impl_abs_diff_eq!(Matrix4, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Matrix4, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Matrix4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let col1 = self.inner.x_axis;
        let col2 = self.inner.y_axis;
        let col3 = self.inner.z_axis;
        let col4 = self.inner.w_axis;
        f.debug_struct("Matrix4")
            .field("column_1", &[col1.x, col1.y, col1.z, col1.w])
            .field("column_2", &[col2.x, col2.y, col2.z, col2.w])
            .field("column_3", &[col3.x, col3.y, col3.z, col3.w])
            .field("column_4", &[col4.x, col4.y, col4.z, col4.w])
            .finish()
    }
}

impl Matrix4C {
    /// Creates the identity matrix.
    #[inline]
    pub const fn identity() -> Self {
        Self::from_columns(
            Vector4C::unit_x(),
            Vector4C::unit_y(),
            Vector4C::unit_z(),
            Vector4C::unit_w(),
        )
    }

    /// Creates a matrix with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::from_columns(
            Vector4C::zeros(),
            Vector4C::zeros(),
            Vector4C::zeros(),
            Vector4C::zeros(),
        )
    }

    /// Creates a diagonal matrix with the given vector as the diagonal.
    #[inline]
    pub const fn from_diagonal(diagonal: &Vector4C) -> Self {
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
        column_1: Vector4C,
        column_2: Vector4C,
        column_3: Vector4C,
        column_4: Vector4C,
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
    pub const fn column_1(&self) -> &Vector4C {
        &self.column_1
    }

    /// The second column of the matrix.
    #[inline]
    pub const fn column_2(&self) -> &Vector4C {
        &self.column_2
    }

    /// The third column of the matrix.
    #[inline]
    pub const fn column_3(&self) -> &Vector4C {
        &self.column_3
    }

    /// The fourth column of the matrix.
    #[inline]
    pub const fn column_4(&self) -> &Vector4C {
        &self.column_4
    }

    /// Sets the first column of the matrix to the given column.
    #[inline]
    pub const fn set_column_1(&mut self, column: Vector4C) {
        self.column_1 = column;
    }

    /// Sets the second column of the matrix to the given column.
    #[inline]
    pub const fn set_column_2(&mut self, column: Vector4C) {
        self.column_2 = column;
    }

    /// Sets the third column of the matrix to the given column.
    #[inline]
    pub const fn set_column_3(&mut self, column: Vector4C) {
        self.column_3 = column;
    }

    /// Sets the fourth column of the matrix to the given column.
    #[inline]
    pub const fn set_column_4(&mut self, column: Vector4C) {
        self.column_4 = column;
    }

    /// Converts the matrix to the 16-byte aligned SIMD-friendly [`Matrix4`].
    #[inline]
    pub fn aligned(&self) -> Matrix4 {
        Matrix4::from_columns(
            self.column_1().aligned(),
            self.column_2().aligned(),
            self.column_3().aligned(),
            self.column_4().aligned(),
        )
    }
}

impl From<Matrix4C> for [f32; 16] {
    fn from(m: Matrix4C) -> [f32; 16] {
        [
            m.column_1.x(),
            m.column_1.y(),
            m.column_1.z(),
            m.column_1.w(),
            m.column_2.x(),
            m.column_2.y(),
            m.column_2.z(),
            m.column_2.w(),
            m.column_3.x(),
            m.column_3.y(),
            m.column_3.z(),
            m.column_3.w(),
            m.column_4.x(),
            m.column_4.y(),
            m.column_4.z(),
            m.column_4.w(),
        ]
    }
}

impl From<[f32; 16]> for Matrix4C {
    fn from(arr: [f32; 16]) -> Matrix4C {
        Matrix4C::from_columns(
            Vector4C::new(arr[0], arr[1], arr[2], arr[3]),
            Vector4C::new(arr[4], arr[5], arr[6], arr[7]),
            Vector4C::new(arr[8], arr[9], arr[10], arr[11]),
            Vector4C::new(arr[12], arr[13], arr[14], arr[15]),
        )
    }
}

impl_abs_diff_eq!(Matrix4C, |a, b, epsilon| {
    a.column_1.abs_diff_eq(&b.column_1, epsilon)
        && a.column_2.abs_diff_eq(&b.column_2, epsilon)
        && a.column_3.abs_diff_eq(&b.column_3, epsilon)
        && a.column_4.abs_diff_eq(&b.column_4, epsilon)
});

impl_relative_eq!(Matrix4C, |a, b, epsilon, max_relative| {
    a.column_1.relative_eq(&b.column_1, epsilon, max_relative)
        && a.column_2.relative_eq(&b.column_2, epsilon, max_relative)
        && a.column_3.relative_eq(&b.column_3, epsilon, max_relative)
        && a.column_4.relative_eq(&b.column_4, epsilon, max_relative)
});

impl_roc_for_library_provided_primitives! {
//  Type        Pkg   Parents  Module   Roc name  Postfix  Precision
    Matrix3C => core, None,    Matrix3, Matrix3,  None,    PrecisionIrrelevant,
    Matrix4C => core, None,    Matrix4, Matrix4,  None,    PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use approx::assert_abs_diff_eq;

    const EPSILON: f32 = 1e-6;

    // === Matrix3 Tests (SIMD-aligned) ===

    #[test]
    fn matrix3_diagonal_extraction_works() {
        let diag_vec = Vector3::new(2.0, 3.0, 4.0);
        let matrix = Matrix3::from_diagonal(&diag_vec);
        let extracted_diag = matrix.diagonal();
        assert_eq!(extracted_diag, diag_vec);
    }

    #[test]
    fn matrix3_transpose_swaps_rows_and_columns() {
        let col1 = Vector3::new(1.0, 2.0, 3.0);
        let col2 = Vector3::new(4.0, 5.0, 6.0);
        let col3 = Vector3::new(7.0, 8.0, 9.0);
        let matrix = Matrix3::from_columns(col1, col2, col3);

        let transpose = matrix.transpose();

        assert_eq!(transpose.element(0, 0), 1.0);
        assert_eq!(transpose.element(0, 1), 2.0);
        assert_eq!(transpose.element(0, 2), 3.0);
        assert_eq!(transpose.element(1, 0), 4.0);
        assert_eq!(transpose.element(1, 1), 5.0);
        assert_eq!(transpose.element(1, 2), 6.0);
        assert_eq!(transpose.element(2, 0), 7.0);
        assert_eq!(transpose.element(2, 1), 8.0);
        assert_eq!(transpose.element(2, 2), 9.0);
    }

    #[test]
    fn matrix3_transpose_is_involutive() {
        let matrix = Matrix3::from_columns(
            Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(4.0, 5.0, 6.0),
            Vector3::new(7.0, 8.0, 9.0),
        );

        let double_transpose = matrix.transpose().transpose();
        assert_eq!(double_transpose, matrix);
    }

    #[test]
    fn matrix3_transpose_of_diagonal_gives_same_matrix() {
        let diag = Vector3::new(1.0, 2.0, 3.0);
        let matrix = Matrix3::from_diagonal(&diag);
        let transpose = matrix.transpose();
        assert_eq!(matrix, transpose);
    }

    #[test]
    fn matrix3_inversion_of_identity_gives_identity() {
        let identity = Matrix3::identity();
        let inverse = identity.inverse();

        for i in 0..3 {
            for j in 0..3 {
                assert_abs_diff_eq!(
                    inverse.element(i, j),
                    identity.element(i, j),
                    epsilon = EPSILON
                );
            }
        }
    }

    #[test]
    fn matrix3_inverse_multiplied_with_original_gives_identity() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));
        let inverse = matrix.inverse();
        let product = &matrix * &inverse;

        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert_abs_diff_eq!(product.element(i, j), expected, epsilon = EPSILON);
            }
        }
    }

    #[test]
    fn matrix3_min_max_element_work() {
        let col1 = Vector3::new(-5.0, 2.0, 0.0);
        let col2 = Vector3::new(3.0, -7.0, 1.0);
        let col3 = Vector3::new(-2.0, 4.0, -3.0);
        let matrix = Matrix3::from_columns(col1, col2, col3);

        assert_abs_diff_eq!(matrix.min_element(), -7.0, epsilon = EPSILON);
        assert_abs_diff_eq!(matrix.max_element(), 4.0, epsilon = EPSILON);
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
    fn matrix3_addition_is_commutative() {
        let m1 = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let m2 = Matrix3::from_diagonal(&Vector3::new(4.0, 5.0, 6.0));

        let result1 = &m1 + &m2;
        let result2 = &m2 + &m1;
        assert_eq!(result1, result2);
    }

    #[test]
    fn matrix3_multiplication_is_associative() {
        let m1 = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));
        let m2 = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let m3 = Matrix3::from_diagonal(&Vector3::new(3.0, 2.0, 1.0));

        let result1 = (&m1 * &m2) * &m3;
        let result2 = &m1 * (&m2 * &m3);

        for i in 0..3 {
            for j in 0..3 {
                assert_abs_diff_eq!(
                    result1.element(i, j),
                    result2.element(i, j),
                    epsilon = EPSILON
                );
            }
        }
    }

    #[test]
    fn matrix3_identity_is_neutral_element() {
        let identity = Matrix3::identity();
        let test_matrix = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));

        let left_mult = &identity * &test_matrix;
        let right_mult = &test_matrix * &identity;

        assert_eq!(left_mult, test_matrix);
        assert_eq!(right_mult, test_matrix);
    }

    #[test]
    fn matrix3_addition_with_zero_gives_same_matrix() {
        let zero = Matrix3::zeros();
        let test_matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));

        let added = &test_matrix + &zero;
        assert_eq!(added, test_matrix);

        let subtracted = &test_matrix - &zero;
        assert_eq!(subtracted, test_matrix);
    }

    #[test]
    fn matrix3_multiplication_with_zero_gives_zero() {
        let zero = Matrix3::zeros();
        let test_matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));

        let multiplied = &test_matrix * &zero;
        for i in 0..3 {
            for j in 0..3 {
                assert_abs_diff_eq!(multiplied.element(i, j), 0.0, epsilon = EPSILON);
            }
        }
    }

    #[test]
    fn matrix3_vector_multiplication_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(2.0, 3.0, 4.0));
        let vector = Vector3::new(1.0, 1.0, 1.0);

        let result = &matrix * &vector;
        assert_eq!(result, Vector3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn matrix3_scalar_multiplication_works() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));

        assert_eq!((&matrix * 2.0).element(0, 0), 2.0);
        assert_eq!((&matrix * 2.0).element(1, 1), 4.0);
        assert_eq!((&matrix * 2.0).element(2, 2), 6.0);

        assert_eq!((3.0 * &matrix).element(0, 0), 3.0);
        assert_eq!((3.0 * &matrix).element(1, 1), 6.0);
        assert_eq!((3.0 * &matrix).element(2, 2), 9.0);
    }

    #[test]
    fn matrix3_scalar_multiplication_by_zero_gives_zero() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let result = &matrix * 0.0;

        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(result.element(i, j), 0.0);
            }
        }
    }

    #[test]
    fn matrix3_scalar_multiplication_by_one_gives_same_matrix() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let result = &matrix * 1.0;
        assert_eq!(result, matrix);
    }

    #[test]
    fn matrix3_scalar_multiplication_by_negative_negates_elements() {
        let matrix = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let result = &matrix * -1.0;

        assert_eq!(result.element(0, 0), -1.0);
        assert_eq!(result.element(1, 1), -2.0);
        assert_eq!(result.element(2, 2), -3.0);
    }

    #[test]
    fn converting_matrix3_to_compact_and_back_preserves_data() {
        let matrix_a = Matrix3::from_diagonal(&Vector3::new(1.0, 2.0, 3.0));
        let matrix = matrix_a.compact();
        assert_eq!(matrix.aligned(), matrix_a);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn indexing_matrix3_element_out_of_bounds_panics() {
        let matrix = Matrix3::identity();
        let _ = matrix.element(3, 0);
    }

    // === Matrix3C Tests (compact) ===

    #[test]
    fn matrix3p_column_accessors_work() {
        let col1 = Vector3C::new(1.0, 2.0, 3.0);
        let col2 = Vector3C::new(4.0, 5.0, 6.0);
        let col3 = Vector3C::new(7.0, 8.0, 9.0);
        let matrix = Matrix3C::from_columns(col1, col2, col3);

        assert_eq!(*matrix.column_1(), col1);
        assert_eq!(*matrix.column_2(), col2);
        assert_eq!(*matrix.column_3(), col3);
    }

    #[test]
    fn matrix3p_column_setters_work() {
        let mut matrix = Matrix3C::zeros();
        let col1 = Vector3C::new(1.0, 2.0, 3.0);
        let col2 = Vector3C::new(4.0, 5.0, 6.0);
        let col3 = Vector3C::new(7.0, 8.0, 9.0);

        matrix.set_column_1(col1);
        matrix.set_column_2(col2);
        matrix.set_column_3(col3);

        assert_eq!(matrix, Matrix3C::from_columns(col1, col2, col3));
    }

    #[test]
    fn matrix3p_from_array_conversion_works() {
        let arr = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let matrix = Matrix3C::from(arr);
        assert_eq!(
            matrix,
            Matrix3C::from_columns(
                Vector3C::new(1.0, 2.0, 3.0),
                Vector3C::new(4.0, 5.0, 6.0),
                Vector3C::new(7.0, 8.0, 9.0)
            )
        );
    }

    #[test]
    fn matrix3p_to_array_conversion_works() {
        let matrix = Matrix3C::from_columns(
            Vector3C::new(1.0, 2.0, 3.0),
            Vector3C::new(4.0, 5.0, 6.0),
            Vector3C::new(7.0, 8.0, 9.0),
        );
        let arr: [f32; 9] = matrix.into();
        assert_eq!(arr, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
    }

    #[test]
    fn converting_matrix3p_to_aligned_and_back_preserves_data() {
        let matrix = Matrix3C::from_diagonal(&Vector3C::new(1.0, 2.0, 3.0));
        let aligned = matrix.aligned();
        assert_eq!(aligned.compact(), matrix);
    }

    // === Matrix4 Tests (SIMD-aligned) ===

    #[test]
    fn matrix4_linear_part_extracts_upper_left_3x3() {
        let col1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4::from_columns(col1, col2, col3, col4);

        let linear = matrix.linear_part();

        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(linear.element(i, j), matrix.element(i, j));
            }
        }
    }

    #[test]
    fn matrix4_translate_transform_sets_translation_column() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(1.0, 2.0, 3.0);

        matrix.translate_transform(&translation);

        assert_eq!(matrix.element(0, 3), 1.0);
        assert_eq!(matrix.element(1, 3), 2.0);
        assert_eq!(matrix.element(2, 3), 3.0);
    }

    #[test]
    fn matrix4_transform_point_applies_translation() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(1.0, 2.0, 3.0);
        matrix.translate_transform(&translation);

        let point = Point3::new(0.0, 0.0, 0.0);
        let transformed = matrix.transform_point(&point);

        assert_eq!(transformed, Point3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn matrix4_transform_vector_ignores_translation() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(5.0, 5.0, 5.0);
        matrix.translate_transform(&translation);

        let vector = Vector3::new(1.0, 1.0, 1.0);
        let point = Point3::new(1.0, 1.0, 1.0);

        let transformed_vector = matrix.transform_vector(&vector);
        let transformed_point = matrix.transform_point(&point);

        assert_eq!(transformed_vector, vector);
        assert_eq!(transformed_point, Point3::new(6.0, 6.0, 6.0));
    }

    #[test]
    fn matrix4_transform_composition_works() {
        let mut matrix = Matrix4::identity();
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let scale = 2.0;

        matrix.translate_transform(&translation);
        matrix.scale_transform(scale);

        let point = Point3::new(1.0, 1.0, 1.0);
        let transformed = matrix.transform_point(&point);

        assert_abs_diff_eq!(transformed.x(), 4.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.y(), 6.0, epsilon = EPSILON);
        assert_abs_diff_eq!(transformed.z(), 8.0, epsilon = EPSILON);
    }

    #[test]
    fn matrix4_identity_is_neutral_element() {
        let identity = Matrix4::identity();
        let test_matrix = Matrix4::from_diagonal(&Vector4::new(2.0, 3.0, 4.0, 5.0));

        let left_mult = &identity * &test_matrix;
        let right_mult = &test_matrix * &identity;

        assert_eq!(left_mult, test_matrix);
        assert_eq!(right_mult, test_matrix);
    }

    #[test]
    fn converting_matrix4_to_compact_and_back_preserves_data() {
        let matrix_a = Matrix4::from_diagonal(&Vector4::new(1.0, 2.0, 3.0, 4.0));
        let matrix = matrix_a.compact();
        assert_eq!(matrix.aligned(), matrix_a);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn indexing_matrix4_element_out_of_bounds_panics() {
        let matrix = Matrix4::identity();
        let _ = matrix.element(4, 0);
    }

    // === Matrix4C Tests (compact) ===

    #[test]
    fn matrix4p_column_accessors_work() {
        let col1 = Vector4C::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4C::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4C::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4C::new(13.0, 14.0, 15.0, 16.0);
        let matrix = Matrix4C::from_columns(col1, col2, col3, col4);

        assert_eq!(*matrix.column_1(), col1);
        assert_eq!(*matrix.column_2(), col2);
        assert_eq!(*matrix.column_3(), col3);
        assert_eq!(*matrix.column_4(), col4);
    }

    #[test]
    fn matrix4p_column_setters_work() {
        let mut matrix = Matrix4C::zeros();
        let col1 = Vector4C::new(1.0, 2.0, 3.0, 4.0);
        let col2 = Vector4C::new(5.0, 6.0, 7.0, 8.0);
        let col3 = Vector4C::new(9.0, 10.0, 11.0, 12.0);
        let col4 = Vector4C::new(13.0, 14.0, 15.0, 16.0);

        matrix.set_column_1(col1);
        matrix.set_column_2(col2);
        matrix.set_column_3(col3);
        matrix.set_column_4(col4);

        assert_eq!(matrix, Matrix4C::from_columns(col1, col2, col3, col4));
    }

    #[test]
    fn matrix4p_from_array_conversion_works() {
        let arr = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let matrix = Matrix4C::from(arr);
        assert_eq!(
            matrix,
            Matrix4C::from_columns(
                Vector4C::new(1.0, 2.0, 3.0, 4.0),
                Vector4C::new(5.0, 6.0, 7.0, 8.0),
                Vector4C::new(9.0, 10.0, 11.0, 12.0),
                Vector4C::new(13.0, 14.0, 15.0, 16.0)
            )
        );
    }

    #[test]
    fn matrix4p_to_array_conversion_works() {
        let matrix = Matrix4C::from_columns(
            Vector4C::new(1.0, 2.0, 3.0, 4.0),
            Vector4C::new(5.0, 6.0, 7.0, 8.0),
            Vector4C::new(9.0, 10.0, 11.0, 12.0),
            Vector4C::new(13.0, 14.0, 15.0, 16.0),
        );
        let arr: [f32; 16] = matrix.into();
        assert_eq!(
            arr,
            [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0
            ]
        );
    }

    #[test]
    fn converting_matrix4p_to_aligned_and_back_preserves_data() {
        let matrix = Matrix4C::from_diagonal(&Vector4C::new(1.0, 2.0, 3.0, 4.0));
        let aligned = matrix.aligned();
        assert_eq!(aligned.compact(), matrix);
    }
}
