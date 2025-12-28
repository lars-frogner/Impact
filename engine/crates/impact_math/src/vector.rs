//! Vectors.

use bytemuck::{Pod, Zeroable};
use core::fmt;
use glam::Vec3Swizzles;
use roc_integration::impl_roc_for_library_provided_primitives;
use std::ops::{Deref, Index, IndexMut, Mul, MulAssign};

/// A 2-dimensional vector.
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Vector2 {
    inner: glam::Vec2,
}

/// A 3-dimensional vector.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Vector3A`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 3]", from = "[f32; 3]")
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

/// A 3-dimensional vector aligned to 16 bytes.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an extra 4 bytes in size and 16-byte alignment.
/// For cache-friendly storage, prefer [`Vector3`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Vector3A {
    inner: glam::Vec3A,
}

/// A 3-dimensional vector of unit length.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`UnitVector3A`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 3]", from = "[f32; 3]")
)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct UnitVector3 {
    x: f32,
    y: f32,
    z: f32,
}

/// A 3-dimensional vector of unit length aligned to 16 bytes.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an extra 4 bytes in size and 16-byte alignment.
/// For cache-friendly storage, prefer [`UnitVector3`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
pub struct UnitVector3A {
    inner: glam::Vec3A,
}

/// A 4-dimensional vector.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`Vector4A`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 4]", from = "[f32; 4]")
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Vector4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

/// A 4-dimensional vector aligned to 16 bytes.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned [`Vector4`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Vector4A {
    inner: glam::Vec4,
}

impl Vector2 {
    /// Creates a new vector with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self::wrap(glam::Vec2::new(x, y))
    }

    /// Creates a new vector with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::wrap(glam::Vec2::ZERO)
    }

    /// Creates a new vector with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::wrap(glam::Vec2::splat(value))
    }

    /// The x-component.
    #[inline]
    pub const fn x(&self) -> f32 {
        self.inner.x
    }

    /// The y-component.
    #[inline]
    pub const fn y(&self) -> f32 {
        self.inner.y
    }

    /// A mutable reference to the x-component.
    #[inline]
    pub const fn x_mut(&mut self) -> &mut f32 {
        &mut self.inner.x
    }

    /// A mutable reference to the y-component.
    #[inline]
    pub const fn y_mut(&mut self) -> &mut f32 {
        &mut self.inner.y
    }

    /// Converts the vector to 3D by appending the given z-component.
    #[inline]
    pub const fn extended(&self, z: f32) -> Vector3 {
        Vector3::new(self.x(), self.y(), z)
    }

    /// Computes the normalized version of the vector.
    #[inline]
    pub fn normalized(&self) -> Self {
        Self::wrap(self.inner.normalize())
    }

    /// Computes the norm (length) of the vector.
    #[inline]
    pub fn norm(&self) -> f32 {
        self.inner.length()
    }

    /// Computes the square of the norm of the vector.
    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.inner.length_squared()
    }

    /// Computes the dot product of this vector with another.
    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.inner.dot(other.inner)
    }

    /// Returns a vector with the absolute value of each component.
    #[inline]
    pub fn component_abs(&self) -> Self {
        Self::wrap(self.inner.abs())
    }

    /// Multiplies each component by the corresponding component in another
    /// vector.
    #[inline]
    pub fn component_mul(&self, other: &Self) -> Self {
        Self::wrap(self.inner * other.inner)
    }

    /// Returns a vector where each component is the minimum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_min(&self, other: &Self) -> Self {
        Self::wrap(self.inner.min(other.inner))
    }

    /// Returns a vector where each component is the maximum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_max(&self, other: &Self) -> Self {
        Self::wrap(self.inner.max(other.inner))
    }

    /// Returns a vector with the given closure applied to each component.
    #[inline]
    pub fn mapped(&self, mut f: impl FnMut(f32) -> f32) -> Self {
        Self::new(f(self.x()), f(self.y()))
    }

    /// Returns the smallest component in the vector.
    #[inline]
    pub fn min_component(&self) -> f32 {
        self.inner.min_element()
    }

    /// Returns the largest component in the vector.
    #[inline]
    pub fn max_component(&self) -> f32 {
        self.inner.max_element()
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Vec2) -> Self {
        Self { inner }
    }

    #[inline]
    pub(crate) const fn unwrap(self) -> glam::Vec2 {
        self.inner
    }
}

impl From<[f32; 2]> for Vector2 {
    #[inline]
    fn from([x, y]: [f32; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<Vector2> for [f32; 2] {
    #[inline]
    fn from(vector: Vector2) -> Self {
        [vector.x(), vector.y()]
    }
}

impl_binop!(Add, add, Vector2, Vector2, Vector2, |a, b| {
    Vector2::wrap(a.inner.add(b.inner))
});

impl_binop!(Sub, sub, Vector2, Vector2, Vector2, |a, b| {
    Vector2::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Vector2, f32, Vector2, |a, b| {
    Vector2::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Vector2, Vector2, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Vector2, f32, Vector2, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Vector2, Vector2, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Vector2, Vector2, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Vector2, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector2, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector2, Vector2, |val| {
    Vector2::wrap(val.inner.neg())
});

impl Index<usize> for Vector2 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector2 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

impl_abs_diff_eq!(Vector2, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Vector2, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vector2")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .finish()
    }
}

impl Vector3 {
    /// Creates a new vector with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Creates a new vector with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::same(0.0)
    }

    /// Creates a new vector with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::new(value, value, value)
    }

    /// The x-axis unit vector.
    #[inline]
    pub const fn unit_x() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    /// The y-axis unit vector.
    #[inline]
    pub const fn unit_y() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    /// The z-axis unit vector.
    #[inline]
    pub const fn unit_z() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }

    /// The x-component.
    #[inline]
    pub const fn x(&self) -> f32 {
        self.x
    }

    /// The y-component.
    #[inline]
    pub const fn y(&self) -> f32 {
        self.y
    }

    /// The z-component.
    #[inline]
    pub const fn z(&self) -> f32 {
        self.z
    }

    /// A mutable reference to the x-component.
    #[inline]
    pub const fn x_mut(&mut self) -> &mut f32 {
        &mut self.x
    }

    /// A mutable reference to the y-component.
    #[inline]
    pub const fn y_mut(&mut self) -> &mut f32 {
        &mut self.y
    }

    /// A mutable reference to the z-component.
    #[inline]
    pub const fn z_mut(&mut self) -> &mut f32 {
        &mut self.z
    }

    /// The 2D vector containing the x- and y-components of this vector.
    #[inline]
    pub const fn xy(&self) -> Vector2 {
        Vector2::new(self.x(), self.y())
    }

    /// Converts the vector to 4D by appending the given w-component.
    #[inline]
    pub const fn extended(&self, w: f32) -> Vector4 {
        Vector4::new(self.x(), self.y(), self.z(), w)
    }

    /// Computes the normalized version of the vector.
    #[inline]
    pub fn normalized(&self) -> Self {
        self / self.norm()
    }

    /// Computes the norm (length) of the vector.
    #[inline]
    pub fn norm(&self) -> f32 {
        self.norm_squared().sqrt()
    }

    /// Computes the square of the norm of the vector.
    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.dot(self)
    }

    /// Computes the dot product of this vector with another.
    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly [`Vector3A`].
    #[inline]
    pub fn aligned(&self) -> Vector3A {
        Vector3A::new(self.x(), self.y(), self.z())
    }

    #[inline]
    pub(crate) const fn from_glam(vector: glam::Vec3) -> Self {
        Self::new(vector.x, vector.y, vector.z)
    }
}

impl From<[f32; 3]> for Vector3 {
    #[inline]
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Vector3> for [f32; 3] {
    #[inline]
    fn from(vector: Vector3) -> Self {
        [vector.x(), vector.y(), vector.z()]
    }
}

impl_binop!(Add, add, Vector3, Vector3, Vector3, |a, b| {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
});

impl_binop!(Sub, sub, Vector3, Vector3, Vector3, |a, b| {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
});

impl_binop!(Mul, mul, Vector3, f32, Vector3, |a, b| {
    Vector3::new(a.x * b, a.y * b, a.z * b)
});

impl_binop!(Mul, mul, f32, Vector3, Vector3, |a, b| { b.mul(a) });

impl_binop!(Div, div, Vector3, f32, Vector3, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Vector3, Vector3, |a, b| {
    a.x += b.x;
    a.y += b.y;
    a.z += b.z;
});

impl_binop_assign!(SubAssign, sub_assign, Vector3, Vector3, |a, b| {
    a.x -= b.x;
    a.y -= b.y;
    a.z -= b.z;
});

impl_binop_assign!(MulAssign, mul_assign, Vector3, f32, |a, b| {
    a.x *= b;
    a.y *= b;
    a.z *= b;
});

impl_binop_assign!(DivAssign, div_assign, Vector3, f32, |a, b| {
    a.mul_assign(b.recip());
});

impl_unary_op!(Neg, neg, Vector3, Vector3, |val| {
    Vector3::new(-val.x, -val.y, -val.z)
});

impl Index<usize> for Vector3 {
    type Output = f32;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        match idx {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("index out of bounds"),
        }
    }
}

impl IndexMut<usize> for Vector3 {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        match idx {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("index out of bounds"),
        }
    }
}

impl_abs_diff_eq!(Vector3, |a, b, epsilon| {
    a.x.abs_diff_eq(&b.x, epsilon)
        && a.y.abs_diff_eq(&b.y, epsilon)
        && a.z.abs_diff_eq(&b.z, epsilon)
});

impl_relative_eq!(Vector3, |a, b, epsilon, max_relative| {
    a.x.relative_eq(&b.x, epsilon, max_relative)
        && a.y.relative_eq(&b.y, epsilon, max_relative)
        && a.z.relative_eq(&b.z, epsilon, max_relative)
});

impl Vector3A {
    /// Creates a new vector with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self::wrap(glam::Vec3A::new(x, y, z))
    }

    /// Creates a new vector with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::wrap(glam::Vec3A::ZERO)
    }

    /// Creates a new vector with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::wrap(glam::Vec3A::splat(value))
    }

    /// The x-axis unit vector.
    #[inline]
    pub const fn unit_x() -> Self {
        Self::wrap(glam::Vec3A::X)
    }

    /// The y-axis unit vector.
    #[inline]
    pub const fn unit_y() -> Self {
        Self::wrap(glam::Vec3A::Y)
    }

    /// The z-axis unit vector.
    #[inline]
    pub const fn unit_z() -> Self {
        Self::wrap(glam::Vec3A::Z)
    }

    /// The x-component.
    #[inline]
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    /// The y-component.
    #[inline]
    pub fn y(&self) -> f32 {
        self.inner.y
    }

    /// The z-component.
    #[inline]
    pub fn z(&self) -> f32 {
        self.inner.z
    }

    /// A mutable reference to the x-component.
    #[inline]
    pub fn x_mut(&mut self) -> &mut f32 {
        &mut self.inner.x
    }

    /// A mutable reference to the y-component.
    #[inline]
    pub fn y_mut(&mut self) -> &mut f32 {
        &mut self.inner.y
    }

    /// A mutable reference to the z-component.
    #[inline]
    pub fn z_mut(&mut self) -> &mut f32 {
        &mut self.inner.z
    }

    /// The 2D vector containing the x- and y-components of this vector.
    #[inline]
    pub fn xy(&self) -> Vector2 {
        Vector2::new(self.x(), self.y())
    }

    /// Creates a vector where the x-, y- and z-components are the y-, z- and
    /// x-component of this vector.
    #[inline]
    pub fn yzx(&self) -> Self {
        Self::wrap(self.inner.yzx())
    }

    /// Creates a vector where the x-, y- and z-components are the z-, x- and
    /// y-component of this vector.
    #[inline]
    pub fn zxy(&self) -> Self {
        Self::wrap(self.inner.zxy())
    }

    /// Creates a vector where the x-, y- and z-components are the y-, x- and
    /// x-component of this vector.
    #[inline]
    pub fn yxx(&self) -> Self {
        Self::wrap(self.inner.yxx())
    }

    /// Creates a vector where the x-, y- and z-components are the z-, z- and
    /// y-component of this vector.
    #[inline]
    pub fn zzy(&self) -> Self {
        Self::wrap(self.inner.zzy())
    }

    /// Converts the vector to 4D by appending the given w-component.
    #[inline]
    pub fn extended(&self, w: f32) -> Vector4A {
        Vector4A::new(self.x(), self.y(), self.z(), w)
    }

    /// Computes the normalized version of the vector.
    #[inline]
    pub fn normalized(&self) -> Self {
        Self::wrap(self.inner.normalize())
    }

    /// Computes the norm (length) of the vector.
    #[inline]
    pub fn norm(&self) -> f32 {
        self.inner.length()
    }

    /// Computes the square of the norm of the vector.
    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.inner.length_squared()
    }

    /// Computes the dot product of this vector with another.
    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.inner.dot(other.inner)
    }

    /// Computes the cross product of this vector with another.
    #[inline]
    pub fn cross(&self, other: &Self) -> Self {
        Self::wrap(self.inner.cross(other.inner))
    }

    /// Returns a vector with the absolute value of each component.
    #[inline]
    pub fn component_abs(&self) -> Self {
        Self::wrap(self.inner.abs())
    }

    /// Multiplies each component by the corresponding component in another
    /// vector.
    #[inline]
    pub fn component_mul(&self, other: &Self) -> Self {
        Self::wrap(self.inner * other.inner)
    }

    /// Returns a vector where each component is the minimum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_min(&self, other: &Self) -> Self {
        Self::wrap(self.inner.min(other.inner))
    }

    /// Returns a vector where each component is the maximum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_max(&self, other: &Self) -> Self {
        Self::wrap(self.inner.max(other.inner))
    }

    /// Returns a vector with the given closure applied to each component.
    #[inline]
    pub fn mapped(&self, mut f: impl FnMut(f32) -> f32) -> Self {
        Self::new(f(self.x()), f(self.y()), f(self.z()))
    }

    /// Returns the smallest component in the vector.
    #[inline]
    pub fn min_component(&self) -> f32 {
        self.inner.min_element()
    }

    /// Returns the largest component in the vector.
    #[inline]
    pub fn max_component(&self) -> f32 {
        self.inner.max_element()
    }

    /// Converts the vector to the 4-byte aligned cache-friendly [`Vector3`].
    #[inline]
    pub fn unaligned(&self) -> Vector3 {
        Vector3::new(self.x(), self.y(), self.z())
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Vec3A) -> Self {
        Self { inner }
    }

    #[inline]
    pub(crate) const fn unwrap(self) -> glam::Vec3A {
        self.inner
    }
}

impl From<[f32; 3]> for Vector3A {
    #[inline]
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Vector3A> for [f32; 3] {
    #[inline]
    fn from(vector: Vector3A) -> Self {
        [vector.x(), vector.y(), vector.z()]
    }
}

impl_binop!(Add, add, Vector3A, Vector3A, Vector3A, |a, b| {
    Vector3A::wrap(a.inner.add(b.inner))
});

impl_binop!(Sub, sub, Vector3A, Vector3A, Vector3A, |a, b| {
    Vector3A::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Vector3A, f32, Vector3A, |a, b| {
    Vector3A::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Vector3A, Vector3A, |a, b| { b.mul(a) });

impl_binop!(Div, div, Vector3A, f32, Vector3A, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, Vector3A, Vector3A, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Vector3A, Vector3A, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Vector3A, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector3A, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector3A, Vector3A, |val| {
    Vector3A::wrap(val.inner.neg())
});

impl Index<usize> for Vector3A {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector3A {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

impl_abs_diff_eq!(Vector3A, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Vector3A, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Vector3A {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vector3A")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .finish()
    }
}

impl UnitVector3 {
    /// Creates a vector with the given components. The vector is assumed to be
    /// normalized.
    #[inline]
    pub const fn new_unchecked(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Converts the given vector to a unit vector, assuming it is already
    /// normalized.
    #[inline]
    pub const fn unchecked_from(vector: Vector3) -> Self {
        Self::new_unchecked(vector.x(), vector.y(), vector.z())
    }

    /// The x-axis unit vector.
    #[inline]
    pub const fn unit_x() -> Self {
        Self::new_unchecked(1.0, 0.0, 0.0)
    }

    /// The y-axis unit vector.
    #[inline]
    pub const fn unit_y() -> Self {
        Self::new_unchecked(0.0, 1.0, 0.0)
    }

    /// The z-axis unit vector.
    #[inline]
    pub const fn unit_z() -> Self {
        Self::new_unchecked(0.0, 0.0, 1.0)
    }

    /// The negative x-axis unit vector.
    #[inline]
    pub const fn neg_unit_x() -> Self {
        Self::new_unchecked(-1.0, 0.0, 0.0)
    }

    /// The negative y-axis unit vector.
    #[inline]
    pub const fn neg_unit_y() -> Self {
        Self::new_unchecked(0.0, -1.0, 0.0)
    }

    /// The negative z-axis unit vector.
    #[inline]
    pub const fn neg_unit_z() -> Self {
        Self::new_unchecked(0.0, 0.0, -1.0)
    }

    /// Creates a unit vector by normalizing the given vector. If the vector has
    /// zero length, the result will be non-finite.
    #[inline]
    pub fn normalized_from(vector: Vector3) -> Self {
        Self::unchecked_from(vector.normalized())
    }

    /// Creates a unit vector by normalizing the given vector if its norm
    /// exceeds the given threshold. Otherwise, returns [`None`].
    #[inline]
    pub fn normalized_from_if_above(vector: Vector3, min_norm: f32) -> Option<Self> {
        Self::normalized_from_and_norm_if_above(vector, min_norm).map(|(v, _norm)| v)
    }

    /// Creates a unit vector by normalizing the given vector, and returns both
    /// the vector and the norm. If the norm is zero, the vector will be
    /// non-finite.
    #[inline]
    pub fn normalized_from_and_norm(vector: Vector3) -> (Self, f32) {
        let norm = vector.norm();
        (Self::unchecked_from(vector / norm), norm)
    }

    /// Creates a unit vector by normalizing the given vector if its norm
    /// exceeds the given threshold, and returns both the vector and the norm.
    /// Returns [`None`] if the norm does not exceed the threshold.
    #[inline]
    pub fn normalized_from_and_norm_if_above(
        vector: Vector3,
        min_norm: f32,
    ) -> Option<(Self, f32)> {
        let norm_squared = vector.norm_squared();
        (norm_squared > min_norm.powi(2)).then(|| {
            let norm = norm_squared.sqrt();
            (Self::unchecked_from(vector / norm), norm)
        })
    }

    /// The x-component.
    #[inline]
    pub const fn x(&self) -> f32 {
        self.x
    }

    /// The y-component.
    #[inline]
    pub const fn y(&self) -> f32 {
        self.y
    }

    /// The z-component.
    #[inline]
    pub const fn z(&self) -> f32 {
        self.z
    }

    /// This unit vector as a [`Vector3`].
    #[inline]
    pub fn as_vector(&self) -> &Vector3 {
        self // deref
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly
    /// [`UnitVector3A`].
    #[inline]
    pub fn aligned(&self) -> UnitVector3A {
        UnitVector3A::new_unchecked(self.x(), self.y(), self.z())
    }
}

impl Deref for UnitVector3 {
    type Target = Vector3;

    #[inline]
    fn deref(&self) -> &Self::Target {
        bytemuck::cast_ref(self)
    }
}

impl From<UnitVector3> for [f32; 3] {
    fn from(vector: UnitVector3) -> Self {
        [vector.x(), vector.y(), vector.z()]
    }
}

impl From<[f32; 3]> for UnitVector3 {
    fn from(vector: [f32; 3]) -> Self {
        Self::normalized_from(vector.into())
    }
}

impl_binop!(Mul, mul, UnitVector3, f32, Vector3, |a, b| {
    Vector3::new(a.x * b, a.y * b, a.z * b)
});

impl_binop!(Mul, mul, f32, UnitVector3, Vector3, |a, b| { b.mul(*a) });

impl_binop!(Div, div, UnitVector3, f32, Vector3, |a, b| {
    a.mul(b.recip())
});

impl_unary_op!(Neg, neg, UnitVector3, UnitVector3, |val| {
    UnitVector3::new_unchecked(-val.x, -val.y, -val.z)
});

impl Index<usize> for UnitVector3 {
    type Output = f32;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        match idx {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("index out of bounds"),
        }
    }
}

impl_abs_diff_eq!(UnitVector3, |a, b, epsilon| {
    a.x.abs_diff_eq(&b.x, epsilon)
        && a.y.abs_diff_eq(&b.y, epsilon)
        && a.z.abs_diff_eq(&b.z, epsilon)
});

impl_relative_eq!(UnitVector3, |a, b, epsilon, max_relative| {
    a.x.relative_eq(&b.x, epsilon, max_relative)
        && a.y.relative_eq(&b.y, epsilon, max_relative)
        && a.z.relative_eq(&b.z, epsilon, max_relative)
});

impl UnitVector3A {
    /// Creates a vector with the given components. The vector is assumed to be
    /// normalized.
    #[inline]
    pub const fn new_unchecked(x: f32, y: f32, z: f32) -> Self {
        Self::wrap(glam::Vec3A::new(x, y, z))
    }

    /// Converts the given vector to a unit vector, assuming it is already
    /// normalized.
    #[inline]
    pub const fn unchecked_from(vector: Vector3A) -> Self {
        Self::wrap(vector.unwrap())
    }

    /// The x-axis unit vector.
    #[inline]
    pub const fn unit_x() -> Self {
        Self::wrap(glam::Vec3A::X)
    }

    /// The y-axis unit vector.
    #[inline]
    pub const fn unit_y() -> Self {
        Self::wrap(glam::Vec3A::Y)
    }

    /// The z-axis unit vector.
    #[inline]
    pub const fn unit_z() -> Self {
        Self::wrap(glam::Vec3A::Z)
    }

    /// The negative x-axis unit vector.
    #[inline]
    pub const fn neg_unit_x() -> Self {
        Self::wrap(glam::Vec3A::NEG_X)
    }

    /// The negative y-axis unit vector.
    #[inline]
    pub const fn neg_unit_y() -> Self {
        Self::wrap(glam::Vec3A::NEG_Y)
    }

    /// The negative z-axis unit vector.
    #[inline]
    pub const fn neg_unit_z() -> Self {
        Self::wrap(glam::Vec3A::NEG_Z)
    }

    /// Creates a unit vector by normalizing the given vector. If the vector has
    /// zero length, the result will be non-finite.
    #[inline]
    pub fn normalized_from(vector: Vector3A) -> Self {
        Self::wrap(vector.unwrap().normalize())
    }

    /// Creates a unit vector by normalizing the given vector if its norm
    /// exceeds the given threshold. Otherwise, returns [`None`].
    #[inline]
    pub fn normalized_from_if_above(vector: Vector3A, min_norm: f32) -> Option<Self> {
        Self::normalized_from_and_norm_if_above(vector, min_norm).map(|(v, _norm)| v)
    }

    /// Creates a unit vector by normalizing the given vector, and returns both
    /// the vector and the norm. If the norm is zero, the vector will be
    /// non-finite.
    #[inline]
    pub fn normalized_from_and_norm(vector: Vector3A) -> (Self, f32) {
        let (inner, norm) = vector.unwrap().normalize_and_length();
        (Self::wrap(inner), norm)
    }

    /// Creates a unit vector by normalizing the given vector if its norm
    /// exceeds the given threshold, and returns both the vector and the norm.
    /// Returns [`None`] if the norm does not exceed the threshold.
    #[inline]
    pub fn normalized_from_and_norm_if_above(
        vector: Vector3A,
        min_norm: f32,
    ) -> Option<(Self, f32)> {
        let v = vector.unwrap();
        let norm_squared = v.length_squared();
        (norm_squared > min_norm.powi(2)).then(|| {
            let norm = norm_squared.sqrt();
            (Self::wrap(v / norm), norm)
        })
    }

    /// The x-component.
    #[inline]
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    /// The y-component.
    #[inline]
    pub fn y(&self) -> f32 {
        self.inner.y
    }

    /// The z-component.
    #[inline]
    pub fn z(&self) -> f32 {
        self.inner.z
    }

    /// This unit vector as a [`Vector3A`].
    #[inline]
    pub fn as_vector(&self) -> &Vector3A {
        self // deref
    }

    /// Converts the vector to the 4-byte aligned cache-friendly
    /// [`UnitVector3`].
    #[inline]
    pub fn unaligned(&self) -> UnitVector3 {
        UnitVector3::new_unchecked(self.x(), self.y(), self.z())
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Vec3A) -> Self {
        Self { inner }
    }
}

impl Deref for UnitVector3A {
    type Target = Vector3A;

    #[inline]
    fn deref(&self) -> &Self::Target {
        bytemuck::cast_ref(self)
    }
}

impl_binop!(Mul, mul, UnitVector3A, f32, Vector3A, |a, b| {
    Vector3A::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, UnitVector3A, Vector3A, |a, b| { b.mul(*a) });

impl_binop!(Div, div, UnitVector3A, f32, Vector3A, |a, b| {
    a.mul(b.recip())
});

impl_unary_op!(Neg, neg, UnitVector3A, UnitVector3A, |val| {
    UnitVector3A::wrap(val.inner.neg())
});

impl Index<usize> for UnitVector3A {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl_abs_diff_eq!(UnitVector3A, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(UnitVector3A, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for UnitVector3A {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitVector3A")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .finish()
    }
}

impl Vector4 {
    /// Creates a new vector with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a new vector with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::same(0.0)
    }

    /// Creates a new vector with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    /// The x-axis unit vector.
    #[inline]
    pub const fn unit_x() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }

    /// The y-axis unit vector.
    #[inline]
    pub const fn unit_y() -> Self {
        Self::new(0.0, 1.0, 0.0, 0.0)
    }

    /// The z-axis unit vector.
    #[inline]
    pub const fn unit_z() -> Self {
        Self::new(0.0, 0.0, 1.0, 0.0)
    }

    /// The w-axis unit vector.
    #[inline]
    pub const fn unit_w() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// The x-component.
    #[inline]
    pub const fn x(&self) -> f32 {
        self.x
    }

    /// The y-component.
    #[inline]
    pub const fn y(&self) -> f32 {
        self.y
    }

    /// The z-component.
    #[inline]
    pub const fn z(&self) -> f32 {
        self.z
    }

    /// The w-component.
    #[inline]
    pub const fn w(&self) -> f32 {
        self.w
    }

    /// A mutable reference to the x-component.
    #[inline]
    pub const fn x_mut(&mut self) -> &mut f32 {
        &mut self.x
    }

    /// A mutable reference to the y-component.
    #[inline]
    pub const fn y_mut(&mut self) -> &mut f32 {
        &mut self.y
    }

    /// A mutable reference to the z-component.
    #[inline]
    pub const fn z_mut(&mut self) -> &mut f32 {
        &mut self.z
    }

    /// A mutable reference to the w-component.
    #[inline]
    pub const fn w_mut(&mut self) -> &mut f32 {
        &mut self.w
    }

    /// The 3D vector containing the x-, y-, and z-components of this vector.
    #[inline]
    pub const fn xyz(&self) -> Vector3 {
        Vector3::new(self.x(), self.y(), self.z())
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly [`Vector4A`].
    #[inline]
    pub fn aligned(&self) -> Vector4A {
        Vector4A::new(self.x(), self.y(), self.z(), self.w())
    }
}

impl From<[f32; 4]> for Vector4 {
    #[inline]
    fn from([x, y, z, w]: [f32; 4]) -> Self {
        Self::new(x, y, z, w)
    }
}

impl From<Vector4> for [f32; 4] {
    #[inline]
    fn from(vector: Vector4) -> Self {
        [vector.x(), vector.y(), vector.z(), vector.w()]
    }
}

impl_binop!(Add, add, Vector4, Vector4, Vector4, |a, b| {
    Vector4::new(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
});

impl_binop!(Sub, sub, Vector4, Vector4, Vector4, |a, b| {
    Vector4::new(a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w)
});

impl_binop!(Mul, mul, Vector4, f32, Vector4, |a, b| {
    Vector4::new(a.x * b, a.y * b, a.z * b, a.w * b)
});

impl_binop!(Mul, mul, f32, Vector4, Vector4, |a, b| { b.mul(a) });

impl_binop!(Div, div, Vector4, f32, Vector4, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Vector4, Vector4, |a, b| {
    a.x += b.x;
    a.y += b.y;
    a.z += b.z;
    a.w += b.w;
});

impl_binop_assign!(SubAssign, sub_assign, Vector4, Vector4, |a, b| {
    a.x -= b.x;
    a.y -= b.y;
    a.z -= b.z;
    a.w -= b.w;
});

impl_binop_assign!(MulAssign, mul_assign, Vector4, f32, |a, b| {
    a.x *= b;
    a.y *= b;
    a.z *= b;
    a.w *= b;
});

impl_binop_assign!(DivAssign, div_assign, Vector4, f32, |a, b| {
    a.mul_assign(b.recip());
});

impl_unary_op!(Neg, neg, Vector4, Vector4, |val| {
    Vector4::new(-val.x, -val.y, -val.z, -val.w)
});

impl Index<usize> for Vector4 {
    type Output = f32;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        match idx {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            3 => &self.w,
            _ => panic!("index out of bounds"),
        }
    }
}

impl IndexMut<usize> for Vector4 {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        match idx {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            3 => &mut self.w,
            _ => panic!("index out of bounds"),
        }
    }
}

impl_abs_diff_eq!(Vector4, |a, b, epsilon| {
    a.x.abs_diff_eq(&b.x, epsilon)
        && a.y.abs_diff_eq(&b.y, epsilon)
        && a.z.abs_diff_eq(&b.z, epsilon)
        && a.w.abs_diff_eq(&b.w, epsilon)
});

impl_relative_eq!(Vector4, |a, b, epsilon, max_relative| {
    a.x.relative_eq(&b.x, epsilon, max_relative)
        && a.y.relative_eq(&b.y, epsilon, max_relative)
        && a.z.relative_eq(&b.z, epsilon, max_relative)
        && a.w.relative_eq(&b.w, epsilon, max_relative)
});

impl Vector4A {
    /// Creates a new vector with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self::wrap(glam::Vec4::new(x, y, z, w))
    }

    /// Creates a new vector with all zeros.
    #[inline]
    pub const fn zeros() -> Self {
        Self::wrap(glam::Vec4::ZERO)
    }

    /// Creates a new vector with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::wrap(glam::Vec4::splat(value))
    }

    /// The x-axis unit vector.
    #[inline]
    pub const fn unit_x() -> Self {
        Self::wrap(glam::Vec4::X)
    }

    /// The y-axis unit vector.
    #[inline]
    pub const fn unit_y() -> Self {
        Self::wrap(glam::Vec4::Y)
    }

    /// The z-axis unit vector.
    #[inline]
    pub const fn unit_z() -> Self {
        Self::wrap(glam::Vec4::Z)
    }

    /// The w-axis unit vector.
    #[inline]
    pub const fn unit_w() -> Self {
        Self::wrap(glam::Vec4::W)
    }

    /// The x-component.
    #[inline]
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    /// The y-component.
    #[inline]
    pub fn y(&self) -> f32 {
        self.inner.y
    }

    /// The z-component.
    #[inline]
    pub fn z(&self) -> f32 {
        self.inner.z
    }

    /// The w-component.
    #[inline]
    pub fn w(&self) -> f32 {
        self.inner.w
    }

    /// A mutable reference to the x-component.
    #[inline]
    pub fn x_mut(&mut self) -> &mut f32 {
        &mut self.inner.x
    }

    /// A mutable reference to the y-component.
    #[inline]
    pub fn y_mut(&mut self) -> &mut f32 {
        &mut self.inner.y
    }

    /// A mutable reference to the z-component.
    #[inline]
    pub fn z_mut(&mut self) -> &mut f32 {
        &mut self.inner.z
    }

    /// A mutable reference to the w-component.
    #[inline]
    pub fn w_mut(&mut self) -> &mut f32 {
        &mut self.inner.w
    }

    /// The 3D vector containing the x-, y-, and z-components of this vector.
    #[inline]
    pub fn xyz(&self) -> Vector3A {
        Vector3A::new(self.x(), self.y(), self.z())
    }

    /// Computes the normalized version of the vector.
    #[inline]
    pub fn normalized(&self) -> Self {
        Self::wrap(self.inner.normalize())
    }

    /// Computes the norm (length) of the vector.
    #[inline]
    pub fn norm(&self) -> f32 {
        self.inner.length()
    }

    /// Computes the square of the norm of the vector.
    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.inner.length_squared()
    }

    /// Computes the dot product of this vector with another.
    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.inner.dot(other.inner)
    }

    /// Returns a vector with the absolute value of each component.
    #[inline]
    pub fn component_abs(&self) -> Self {
        Self::wrap(self.inner.abs())
    }

    /// Multiplies each component by the corresponding component in another
    /// vector.
    #[inline]
    pub fn component_mul(&self, other: &Self) -> Self {
        Self::wrap(self.inner * other.inner)
    }

    /// Returns a vector where each component is the minimum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_min(&self, other: &Self) -> Self {
        Self::wrap(self.inner.min(other.inner))
    }

    /// Returns a vector where each component is the maximum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_max(&self, other: &Self) -> Self {
        Self::wrap(self.inner.max(other.inner))
    }

    /// Returns a vector with the given closure applied to each component.
    #[inline]
    pub fn mapped(&self, mut f: impl FnMut(f32) -> f32) -> Self {
        Self::new(f(self.x()), f(self.y()), f(self.z()), f(self.w()))
    }

    /// Returns the smallest component in the vector.
    #[inline]
    pub fn min_component(&self) -> f32 {
        self.inner.min_element()
    }

    /// Returns the largest component in the vector.
    #[inline]
    pub fn max_component(&self) -> f32 {
        self.inner.max_element()
    }

    /// Converts the vector to the 4-byte aligned cache-friendly [`Vector4`].
    #[inline]
    pub fn unaligned(&self) -> Vector4 {
        Vector4::new(self.x(), self.y(), self.z(), self.w())
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Vec4) -> Self {
        Self { inner }
    }

    #[inline]
    pub(crate) const fn unwrap(self) -> glam::Vec4 {
        self.inner
    }
}

impl From<[f32; 4]> for Vector4A {
    #[inline]
    fn from([x, y, z, w]: [f32; 4]) -> Self {
        Self::new(x, y, z, w)
    }
}

impl From<Vector4A> for [f32; 4] {
    #[inline]
    fn from(vector: Vector4A) -> Self {
        [vector.x(), vector.y(), vector.z(), vector.w()]
    }
}

impl_binop!(Add, add, Vector4A, Vector4A, Vector4A, |a, b| {
    Vector4A::wrap(a.inner.add(b.inner))
});

impl_binop!(Sub, sub, Vector4A, Vector4A, Vector4A, |a, b| {
    Vector4A::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Vector4A, f32, Vector4A, |a, b| {
    Vector4A::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Vector4A, Vector4A, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Vector4A, f32, Vector4A, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, Vector4A, Vector4A, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Vector4A, Vector4A, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Vector4A, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector4A, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector4A, Vector4A, |val| {
    Vector4A::wrap(val.inner.neg())
});

impl Index<usize> for Vector4A {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector4A {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

impl_abs_diff_eq!(Vector4A, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Vector4A, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Vector4A {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vector4A")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .field("w", &self.inner.w)
            .finish()
    }
}

impl_roc_for_library_provided_primitives! {
//  Type           Pkg   Parents  Module       Roc name     Postfix  Precision
    Vector2     => core, None,    Vector2,     Vector2,     None,    PrecisionIrrelevant,
    Vector3     => core, None,    Vector3,     Vector3,     None,    PrecisionIrrelevant,
    UnitVector3 => core, None,    UnitVector3, UnitVector3, None,    PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use approx::assert_abs_diff_eq;

    // Test constants
    const EPSILON: f32 = 1e-6;

    #[test]
    fn vector2_new_works() {
        let v = Vector2::new(1.0, 2.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
    }

    #[test]
    fn vector2_zeros_gives_zero_vector() {
        let v = Vector2::zeros();
        assert_eq!(v.x(), 0.0);
        assert_eq!(v.y(), 0.0);
    }

    #[test]
    fn vector2_same_creates_vector_with_repeated_value() {
        let v = Vector2::same(3.5);
        assert_eq!(v.x(), 3.5);
        assert_eq!(v.y(), 3.5);
    }

    #[test]
    fn vector2_component_accessors_work() {
        let mut v = Vector2::new(1.0, 2.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);

        *v.x_mut() = 10.0;
        *v.y_mut() = 20.0;
        assert_eq!(v.x(), 10.0);
        assert_eq!(v.y(), 20.0);
    }

    #[test]
    fn vector2_norm_calculations_work() {
        let v = Vector2::new(3.0, 4.0);
        assert_abs_diff_eq!(v.norm(), 5.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 25.0, epsilon = EPSILON);
    }

    #[test]
    fn vector2_normalized_gives_unit_vector() {
        let v = Vector2::new(3.0, 4.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.x(), 0.6, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.y(), 0.8, epsilon = EPSILON);
    }

    #[test]
    fn vector2_dot_product_works() {
        let v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);
        assert_abs_diff_eq!(v1.dot(&v2), 11.0, epsilon = EPSILON);
    }

    #[test]
    fn vector2_component_operations_work() {
        let v1 = Vector2::new(-1.0, 2.0);
        let v2 = Vector2::new(3.0, -4.0);

        let abs_v = v1.component_abs();
        assert_eq!(abs_v.x(), 1.0);
        assert_eq!(abs_v.y(), 2.0);

        let mul_v = v1.component_mul(&v2);
        assert_eq!(mul_v.x(), -3.0);
        assert_eq!(mul_v.y(), -8.0);

        let min_v = v1.component_min(&v2);
        assert_eq!(min_v.x(), -1.0);
        assert_eq!(min_v.y(), -4.0);

        let max_v = v1.component_max(&v2);
        assert_eq!(max_v.x(), 3.0);
        assert_eq!(max_v.y(), 2.0);
    }

    #[test]
    fn vector2_max_component_returns_largest_element() {
        let v = Vector2::new(1.5, 3.7);
        assert_abs_diff_eq!(v.max_component(), 3.7, epsilon = EPSILON);
    }

    #[test]
    fn vector2_arithmetic_operations_work() {
        let v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);

        let add_result = &v1 + &v2;
        assert_eq!(add_result.x(), 4.0);
        assert_eq!(add_result.y(), 6.0);

        let sub_result = &v1 - &v2;
        assert_eq!(sub_result.x(), -2.0);
        assert_eq!(sub_result.y(), -2.0);

        let mul_scalar = &v1 * 2.0;
        assert_eq!(mul_scalar.x(), 2.0);
        assert_eq!(mul_scalar.y(), 4.0);

        let scalar_mul = 3.0 * &v1;
        assert_eq!(scalar_mul.x(), 3.0);
        assert_eq!(scalar_mul.y(), 6.0);

        let div_scalar = &v1 / 2.0;
        assert_eq!(div_scalar.x(), 0.5);
        assert_eq!(div_scalar.y(), 1.0);

        let neg_v = -&v1;
        assert_eq!(neg_v.x(), -1.0);
        assert_eq!(neg_v.y(), -2.0);
    }

    #[test]
    fn vector2_assignment_operations_work() {
        let mut v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);

        v1 += &v2;
        assert_eq!(v1.x(), 4.0);
        assert_eq!(v1.y(), 6.0);

        v1 -= &v2;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);

        v1 *= 2.0;
        assert_eq!(v1.x(), 2.0);
        assert_eq!(v1.y(), 4.0);

        v1 /= 2.0;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
    }

    #[test]
    fn vector2_indexing_works() {
        let mut v = Vector2::new(1.0, 2.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);

        v[0] = 10.0;
        v[1] = 20.0;
        assert_eq!(v[0], 10.0);
        assert_eq!(v[1], 20.0);
    }

    #[test]
    fn vector2_array_conversion_works() {
        let arr: [f32; 2] = [1.0, 2.0];
        let v = Vector2::from(arr);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);

        let converted_back: [f32; 2] = v.into();
        assert_eq!(converted_back, [1.0, 2.0]);
    }

    #[test]
    fn vector3a_new_works() {
        let v = Vector3A::new(1.0, 2.0, 3.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
    }

    #[test]
    fn vector3a_unit_vectors_work() {
        let unit_x = Vector3A::unit_x();
        assert_eq!(unit_x.x(), 1.0);
        assert_eq!(unit_x.y(), 0.0);
        assert_eq!(unit_x.z(), 0.0);

        let unit_y = Vector3A::unit_y();
        assert_eq!(unit_y.x(), 0.0);
        assert_eq!(unit_y.y(), 1.0);
        assert_eq!(unit_y.z(), 0.0);

        let unit_z = Vector3A::unit_z();
        assert_eq!(unit_z.x(), 0.0);
        assert_eq!(unit_z.y(), 0.0);
        assert_eq!(unit_z.z(), 1.0);
    }

    #[test]
    fn vector3a_cross_product_works() {
        let v1 = Vector3A::new(1.0, 0.0, 0.0);
        let v2 = Vector3A::new(0.0, 1.0, 0.0);
        let cross = v1.cross(&v2);
        assert_abs_diff_eq!(cross.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.z(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3a_cross_product_is_perpendicular() {
        let v1 = Vector3A::new(1.0, 2.0, 3.0);
        let v2 = Vector3A::new(4.0, 5.0, 6.0);
        let cross = v1.cross(&v2);

        // Cross product should be perpendicular to both input vectors
        assert_abs_diff_eq!(cross.dot(&v1), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.dot(&v2), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3a_cross_product_is_anticommutative() {
        let v1 = Vector3A::new(1.0, 2.0, 3.0);
        let v2 = Vector3A::new(4.0, 5.0, 6.0);
        let cross1 = v1.cross(&v2);
        let cross2 = v2.cross(&v1);

        // v1 × v2 = -(v2 × v1)
        assert_abs_diff_eq!(cross1.x(), -cross2.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(cross1.y(), -cross2.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(cross1.z(), -cross2.z(), epsilon = EPSILON);
    }

    #[test]
    fn vector3a_cross_product_of_parallel_vectors_is_zero() {
        let v1 = Vector3A::new(1.0, 2.0, 3.0);
        let v2 = Vector3A::new(2.0, 4.0, 6.0); // Parallel to v1
        let cross = v1.cross(&v2);

        assert_abs_diff_eq!(cross.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3a_xy_extraction_works() {
        let v3 = Vector3A::new(1.0, 2.0, 3.0);
        let xy = v3.xy();
        assert_eq!(xy.x(), 1.0);
        assert_eq!(xy.y(), 2.0);
    }

    #[test]
    fn vector4a_new_works() {
        let v = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.w(), 4.0);
    }

    #[test]
    fn vector4a_xyz_extraction_works() {
        let v4 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let xyz = v4.xyz();
        assert_eq!(xyz.x(), 1.0);
        assert_eq!(xyz.y(), 2.0);
        assert_eq!(xyz.z(), 3.0);
    }

    #[test]
    fn vector4a_component_mutators_work() {
        let mut v = Vector4A::new(1.0, 2.0, 3.0, 4.0);

        *v.x_mut() = 10.0;
        *v.y_mut() = 20.0;
        *v.z_mut() = 30.0;
        *v.w_mut() = 40.0;

        assert_eq!(v.x(), 10.0);
        assert_eq!(v.y(), 20.0);
        assert_eq!(v.z(), 30.0);
        assert_eq!(v.w(), 40.0);
    }

    #[test]
    fn unitvector3a_unit_vectors_work() {
        let unit_x = UnitVector3A::unit_x();
        assert_eq!(unit_x.x(), 1.0);
        assert_eq!(unit_x.y(), 0.0);
        assert_eq!(unit_x.z(), 0.0);
        assert_abs_diff_eq!(unit_x.norm(), 1.0, epsilon = EPSILON);

        let unit_y = UnitVector3A::unit_y();
        assert_eq!(unit_y.x(), 0.0);
        assert_eq!(unit_y.y(), 1.0);
        assert_eq!(unit_y.z(), 0.0);
        assert_abs_diff_eq!(unit_y.norm(), 1.0, epsilon = EPSILON);

        let unit_z = UnitVector3A::unit_z();
        assert_eq!(unit_z.x(), 0.0);
        assert_eq!(unit_z.y(), 0.0);
        assert_eq!(unit_z.z(), 1.0);
        assert_abs_diff_eq!(unit_z.norm(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3a_normalized_from_works() {
        let v = Vector3A::new(3.0, 4.0, 0.0);
        let unit = UnitVector3A::normalized_from(v);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.x(), 0.6, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.y(), 0.8, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3a_normalized_from_if_above_works() {
        let v_large = Vector3A::new(3.0, 4.0, 0.0);
        let unit_large = UnitVector3A::normalized_from_if_above(v_large, 1.0);
        assert!(unit_large.is_some());
        let unit = unit_large.unwrap();
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);

        let v_small = Vector3A::new(0.1, 0.1, 0.0);
        let unit_small = UnitVector3A::normalized_from_if_above(v_small, 1.0);
        assert!(unit_small.is_none());
    }

    #[test]
    fn unitvector3a_normalized_from_and_norm_works() {
        let v = Vector3A::new(3.0, 4.0, 0.0);
        let (unit, norm) = UnitVector3A::normalized_from_and_norm(v);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(norm, 5.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3a_normalized_from_and_norm_if_above_works() {
        let v_large = Vector3A::new(3.0, 4.0, 0.0);
        let result_large = UnitVector3A::normalized_from_and_norm_if_above(v_large, 1.0);
        assert!(result_large.is_some());
        let (unit, norm) = result_large.unwrap();
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(norm, 5.0, epsilon = EPSILON);

        let v_small = Vector3A::new(0.1, 0.1, 0.0);
        let result_small = UnitVector3A::normalized_from_and_norm_if_above(v_small, 1.0);
        assert!(result_small.is_none());
    }

    #[test]
    fn unitvector3a_unchecked_from_works() {
        let v = Vector3A::new(1.0, 0.0, 0.0); // Already normalized
        let unit = UnitVector3A::unchecked_from(v);
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
    }

    #[test]
    fn unitvector3a_as_vector_works() {
        let unit = UnitVector3A::unit_x();
        let as_vec = unit.as_vector();
        assert_eq!(as_vec.x(), 1.0);
        assert_eq!(as_vec.y(), 0.0);
        assert_eq!(as_vec.z(), 0.0);
    }

    #[test]
    fn unitvector3a_deref_to_vector3a_works() {
        let unit = UnitVector3A::unit_x();
        // Test that UnitVector3A can be used as Vector3A through Deref
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3a_indexing_works() {
        let unit = UnitVector3A::unit_y();
        assert_eq!(unit[0], 0.0);
        assert_eq!(unit[1], 1.0);
        assert_eq!(unit[2], 0.0);
    }

    #[test]
    fn unitvector3a_arithmetic_with_scalar_works() {
        let unit = UnitVector3A::unit_x();
        let scaled = &unit * 2.0;
        assert_eq!(scaled.x(), 2.0);
        assert_eq!(scaled.y(), 0.0);
        assert_eq!(scaled.z(), 0.0);
    }

    // Vector2 missing methods tests
    #[test]
    fn vector2_extended_works() {
        let v2 = Vector2::new(1.0, 2.0);
        let v3 = v2.extended(3.0);
        assert_eq!(v3.x(), 1.0);
        assert_eq!(v3.y(), 2.0);
        assert_eq!(v3.z(), 3.0);
    }

    #[test]
    fn vector2_mapped_transforms_components() {
        let v = Vector2::new(1.0, -2.0);
        let mapped = v.mapped(|x| x * 2.0);
        assert_eq!(mapped.x(), 2.0);
        assert_eq!(mapped.y(), -4.0);
    }

    #[test]
    fn vector2_min_component_returns_smallest_element() {
        let v = Vector2::new(3.7, 1.5);
        assert_abs_diff_eq!(v.min_component(), 1.5, epsilon = EPSILON);
    }

    // Vector3 (non-aligned) tests
    #[test]
    fn vector3_new_works() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
    }

    #[test]
    fn vector3_unit_vectors_work() {
        let unit_x = Vector3::unit_x();
        assert_eq!(unit_x.x(), 1.0);
        assert_eq!(unit_x.y(), 0.0);
        assert_eq!(unit_x.z(), 0.0);

        let unit_y = Vector3::unit_y();
        assert_eq!(unit_y.x(), 0.0);
        assert_eq!(unit_y.y(), 1.0);
        assert_eq!(unit_y.z(), 0.0);

        let unit_z = Vector3::unit_z();
        assert_eq!(unit_z.x(), 0.0);
        assert_eq!(unit_z.y(), 0.0);
        assert_eq!(unit_z.z(), 1.0);
    }

    #[test]
    fn vector3_zeros_gives_zero_vector() {
        let v = Vector3::zeros();
        assert_eq!(v.x(), 0.0);
        assert_eq!(v.y(), 0.0);
        assert_eq!(v.z(), 0.0);
    }

    #[test]
    fn vector3_same_creates_vector_with_repeated_value() {
        let v = Vector3::same(2.5);
        assert_eq!(v.x(), 2.5);
        assert_eq!(v.y(), 2.5);
        assert_eq!(v.z(), 2.5);
    }

    #[test]
    fn vector3_xy_extraction_works() {
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let xy = v3.xy();
        assert_eq!(xy.x(), 1.0);
        assert_eq!(xy.y(), 2.0);
    }

    #[test]
    fn vector3_extended_works() {
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let v4 = v3.extended(4.0);
        assert_eq!(v4.x(), 1.0);
        assert_eq!(v4.y(), 2.0);
        assert_eq!(v4.z(), 3.0);
        assert_eq!(v4.w(), 4.0);
    }

    #[test]
    fn vector3_aligned_conversion_works() {
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let v3a = v3.aligned();
        assert_eq!(v3a.x(), 1.0);
        assert_eq!(v3a.y(), 2.0);
        assert_eq!(v3a.z(), 3.0);
    }

    #[test]
    fn vector3_dot_product_works() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        assert_abs_diff_eq!(v1.dot(&v2), 32.0, epsilon = EPSILON); // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn vector3_component_mutators_work() {
        let mut v = Vector3::new(1.0, 2.0, 3.0);

        *v.x_mut() = 10.0;
        *v.y_mut() = 20.0;
        *v.z_mut() = 30.0;

        assert_eq!(v.x(), 10.0);
        assert_eq!(v.y(), 20.0);
        assert_eq!(v.z(), 30.0);
    }

    #[test]
    fn vector3_indexing_works() {
        let mut v = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);

        v[0] = 10.0;
        v[1] = 20.0;
        v[2] = 30.0;
        assert_eq!(v[0], 10.0);
        assert_eq!(v[1], 20.0);
        assert_eq!(v[2], 30.0);
    }

    #[test]
    fn vector3_array_conversion_works() {
        let arr: [f32; 3] = [1.0, 2.0, 3.0];
        let v = Vector3::from(arr);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);

        let converted_back: [f32; 3] = v.into();
        assert_eq!(converted_back, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn vector3_arithmetic_operations_work() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);

        let add_result = &v1 + &v2;
        assert_eq!(add_result.x(), 5.0);
        assert_eq!(add_result.y(), 7.0);
        assert_eq!(add_result.z(), 9.0);

        let sub_result = &v1 - &v2;
        assert_eq!(sub_result.x(), -3.0);
        assert_eq!(sub_result.y(), -3.0);
        assert_eq!(sub_result.z(), -3.0);

        let mul_scalar = &v1 * 2.0;
        assert_eq!(mul_scalar.x(), 2.0);
        assert_eq!(mul_scalar.y(), 4.0);
        assert_eq!(mul_scalar.z(), 6.0);

        let scalar_mul = 3.0 * &v1;
        assert_eq!(scalar_mul.x(), 3.0);
        assert_eq!(scalar_mul.y(), 6.0);
        assert_eq!(scalar_mul.z(), 9.0);

        let div_scalar = &v1 / 2.0;
        assert_eq!(div_scalar.x(), 0.5);
        assert_eq!(div_scalar.y(), 1.0);
        assert_eq!(div_scalar.z(), 1.5);

        let neg_v = -&v1;
        assert_eq!(neg_v.x(), -1.0);
        assert_eq!(neg_v.y(), -2.0);
        assert_eq!(neg_v.z(), -3.0);
    }

    #[test]
    fn vector3_assignment_operations_work() {
        let mut v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);

        v1 += &v2;
        assert_eq!(v1.x(), 5.0);
        assert_eq!(v1.y(), 7.0);
        assert_eq!(v1.z(), 9.0);

        v1 -= &v2;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);

        v1 *= 2.0;
        assert_eq!(v1.x(), 2.0);
        assert_eq!(v1.y(), 4.0);
        assert_eq!(v1.z(), 6.0);

        v1 /= 2.0;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);
    }

    // Vector4 (non-aligned) tests
    #[test]
    fn vector4_new_works() {
        let v = Vector4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.w(), 4.0);
    }

    #[test]
    fn vector4_unit_vectors_work() {
        let unit_x = Vector4::unit_x();
        assert_eq!(unit_x.x(), 1.0);
        assert_eq!(unit_x.y(), 0.0);
        assert_eq!(unit_x.z(), 0.0);
        assert_eq!(unit_x.w(), 0.0);

        let unit_y = Vector4::unit_y();
        assert_eq!(unit_y.x(), 0.0);
        assert_eq!(unit_y.y(), 1.0);
        assert_eq!(unit_y.z(), 0.0);
        assert_eq!(unit_y.w(), 0.0);

        let unit_z = Vector4::unit_z();
        assert_eq!(unit_z.x(), 0.0);
        assert_eq!(unit_z.y(), 0.0);
        assert_eq!(unit_z.z(), 1.0);
        assert_eq!(unit_z.w(), 0.0);

        let unit_w = Vector4::unit_w();
        assert_eq!(unit_w.x(), 0.0);
        assert_eq!(unit_w.y(), 0.0);
        assert_eq!(unit_w.z(), 0.0);
        assert_eq!(unit_w.w(), 1.0);
    }

    #[test]
    fn vector4_zeros_gives_zero_vector() {
        let v = Vector4::zeros();
        assert_eq!(v.x(), 0.0);
        assert_eq!(v.y(), 0.0);
        assert_eq!(v.z(), 0.0);
        assert_eq!(v.w(), 0.0);
    }

    #[test]
    fn vector4_same_creates_vector_with_repeated_value() {
        let v = Vector4::same(1.5);
        assert_eq!(v.x(), 1.5);
        assert_eq!(v.y(), 1.5);
        assert_eq!(v.z(), 1.5);
        assert_eq!(v.w(), 1.5);
    }

    #[test]
    fn vector4_xyz_extraction_works() {
        let v4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let xyz = v4.xyz();
        assert_eq!(xyz.x(), 1.0);
        assert_eq!(xyz.y(), 2.0);
        assert_eq!(xyz.z(), 3.0);
    }

    #[test]
    fn vector4_aligned_conversion_works() {
        let v4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v4a = v4.aligned();
        assert_eq!(v4a.x(), 1.0);
        assert_eq!(v4a.y(), 2.0);
        assert_eq!(v4a.z(), 3.0);
        assert_eq!(v4a.w(), 4.0);
    }

    #[test]
    fn vector4_component_mutators_work() {
        let mut v = Vector4::new(1.0, 2.0, 3.0, 4.0);

        *v.x_mut() = 10.0;
        *v.y_mut() = 20.0;
        *v.z_mut() = 30.0;
        *v.w_mut() = 40.0;

        assert_eq!(v.x(), 10.0);
        assert_eq!(v.y(), 20.0);
        assert_eq!(v.z(), 30.0);
        assert_eq!(v.w(), 40.0);
    }

    #[test]
    fn vector4_indexing_works() {
        let mut v = Vector4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);
        assert_eq!(v[3], 4.0);

        v[0] = 10.0;
        v[1] = 20.0;
        v[2] = 30.0;
        v[3] = 40.0;
        assert_eq!(v[0], 10.0);
        assert_eq!(v[1], 20.0);
        assert_eq!(v[2], 30.0);
        assert_eq!(v[3], 40.0);
    }

    #[test]
    fn vector4_array_conversion_works() {
        let arr: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        let v = Vector4::from(arr);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.w(), 4.0);

        let converted_back: [f32; 4] = v.into();
        assert_eq!(converted_back, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn vector4_arithmetic_operations_work() {
        let v1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4::new(5.0, 6.0, 7.0, 8.0);

        let add_result = &v1 + &v2;
        assert_eq!(add_result.x(), 6.0);
        assert_eq!(add_result.y(), 8.0);
        assert_eq!(add_result.z(), 10.0);
        assert_eq!(add_result.w(), 12.0);

        let sub_result = &v1 - &v2;
        assert_eq!(sub_result.x(), -4.0);
        assert_eq!(sub_result.y(), -4.0);
        assert_eq!(sub_result.z(), -4.0);
        assert_eq!(sub_result.w(), -4.0);

        let mul_scalar = &v1 * 2.0;
        assert_eq!(mul_scalar.x(), 2.0);
        assert_eq!(mul_scalar.y(), 4.0);
        assert_eq!(mul_scalar.z(), 6.0);
        assert_eq!(mul_scalar.w(), 8.0);

        let scalar_mul = 3.0 * &v1;
        assert_eq!(scalar_mul.x(), 3.0);
        assert_eq!(scalar_mul.y(), 6.0);
        assert_eq!(scalar_mul.z(), 9.0);
        assert_eq!(scalar_mul.w(), 12.0);

        let div_scalar = &v1 / 2.0;
        assert_eq!(div_scalar.x(), 0.5);
        assert_eq!(div_scalar.y(), 1.0);
        assert_eq!(div_scalar.z(), 1.5);
        assert_eq!(div_scalar.w(), 2.0);

        let neg_v = -&v1;
        assert_eq!(neg_v.x(), -1.0);
        assert_eq!(neg_v.y(), -2.0);
        assert_eq!(neg_v.z(), -3.0);
        assert_eq!(neg_v.w(), -4.0);
    }

    #[test]
    fn vector4_assignment_operations_work() {
        let mut v1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4::new(5.0, 6.0, 7.0, 8.0);

        v1 += &v2;
        assert_eq!(v1.x(), 6.0);
        assert_eq!(v1.y(), 8.0);
        assert_eq!(v1.z(), 10.0);
        assert_eq!(v1.w(), 12.0);

        v1 -= &v2;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);
        assert_eq!(v1.w(), 4.0);

        v1 *= 2.0;
        assert_eq!(v1.x(), 2.0);
        assert_eq!(v1.y(), 4.0);
        assert_eq!(v1.z(), 6.0);
        assert_eq!(v1.w(), 8.0);

        v1 /= 2.0;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);
        assert_eq!(v1.w(), 4.0);
    }

    // UnitVector3 (non-aligned) tests
    #[test]
    fn unitvector3_unit_vectors_work() {
        let unit_x = UnitVector3::unit_x();
        assert_eq!(unit_x.x(), 1.0);
        assert_eq!(unit_x.y(), 0.0);
        assert_eq!(unit_x.z(), 0.0);

        let unit_y = UnitVector3::unit_y();
        assert_eq!(unit_y.x(), 0.0);
        assert_eq!(unit_y.y(), 1.0);
        assert_eq!(unit_y.z(), 0.0);

        let unit_z = UnitVector3::unit_z();
        assert_eq!(unit_z.x(), 0.0);
        assert_eq!(unit_z.y(), 0.0);
        assert_eq!(unit_z.z(), 1.0);
    }

    #[test]
    fn unitvector3_negative_unit_vectors_work() {
        let neg_unit_x = UnitVector3::neg_unit_x();
        assert_eq!(neg_unit_x.x(), -1.0);
        assert_eq!(neg_unit_x.y(), 0.0);
        assert_eq!(neg_unit_x.z(), 0.0);

        let neg_unit_y = UnitVector3::neg_unit_y();
        assert_eq!(neg_unit_y.x(), 0.0);
        assert_eq!(neg_unit_y.y(), -1.0);
        assert_eq!(neg_unit_y.z(), 0.0);

        let neg_unit_z = UnitVector3::neg_unit_z();
        assert_eq!(neg_unit_z.x(), 0.0);
        assert_eq!(neg_unit_z.y(), 0.0);
        assert_eq!(neg_unit_z.z(), -1.0);
    }

    #[test]
    fn unitvector3_as_vector_works() {
        let unit = UnitVector3::unit_x();
        let as_vec = unit.as_vector();
        assert_eq!(as_vec.x(), 1.0);
        assert_eq!(as_vec.y(), 0.0);
        assert_eq!(as_vec.z(), 0.0);
    }

    #[test]
    fn unitvector3_aligned_conversion_works() {
        let unit = UnitVector3::unit_x();
        let unit_a = unit.aligned();
        assert_eq!(unit_a.x(), 1.0);
        assert_eq!(unit_a.y(), 0.0);
        assert_eq!(unit_a.z(), 0.0);
    }

    #[test]
    fn unitvector3_deref_to_vector3_works() {
        let unit = UnitVector3::unit_x();
        // Test that UnitVector3 can be used as Vector3 through Deref
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
    }

    #[test]
    fn unitvector3_indexing_works() {
        let unit = UnitVector3::unit_y();
        assert_eq!(unit[0], 0.0);
        assert_eq!(unit[1], 1.0);
        assert_eq!(unit[2], 0.0);
    }

    #[test]
    fn unitvector3_new_unchecked_works() {
        let unit = UnitVector3::new_unchecked(1.0, 0.0, 0.0); // Already normalized
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
    }

    #[test]
    fn unitvector3_normalized_from_works() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let unit = UnitVector3::normalized_from(v);

        // Should normalize to unit length
        let norm = (unit.x() * unit.x() + unit.y() * unit.y() + unit.z() * unit.z()).sqrt();
        assert_abs_diff_eq!(norm, 1.0, epsilon = EPSILON);

        // Should maintain direction (parallel to original)
        assert_abs_diff_eq!(unit.x(), 3.0 / 5.0, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.y(), 4.0 / 5.0, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3a_negative_unit_vectors_work() {
        let neg_unit_x = UnitVector3A::neg_unit_x();
        assert_eq!(neg_unit_x.x(), -1.0);
        assert_eq!(neg_unit_x.y(), 0.0);
        assert_eq!(neg_unit_x.z(), 0.0);

        let neg_unit_y = UnitVector3A::neg_unit_y();
        assert_eq!(neg_unit_y.x(), 0.0);
        assert_eq!(neg_unit_y.y(), -1.0);
        assert_eq!(neg_unit_y.z(), 0.0);

        let neg_unit_z = UnitVector3A::neg_unit_z();
        assert_eq!(neg_unit_z.x(), 0.0);
        assert_eq!(neg_unit_z.y(), 0.0);
        assert_eq!(neg_unit_z.z(), -1.0);
    }

    #[test]
    fn unitvector3a_unaligned_conversion_works() {
        let unit_a = UnitVector3A::unit_x();
        let unit = unit_a.unaligned();
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
    }

    // Additional Vector3A tests for complete coverage
    #[test]
    fn vector3a_zeros_gives_zero_vector() {
        let v = Vector3A::zeros();
        assert_eq!(v.x(), 0.0);
        assert_eq!(v.y(), 0.0);
        assert_eq!(v.z(), 0.0);
    }

    #[test]
    fn vector3a_same_creates_vector_with_repeated_value() {
        let v = Vector3A::same(2.5);
        assert_eq!(v.x(), 2.5);
        assert_eq!(v.y(), 2.5);
        assert_eq!(v.z(), 2.5);
    }

    #[test]
    fn vector3a_extended_works() {
        let v3 = Vector3A::new(1.0, 2.0, 3.0);
        let v4 = v3.extended(4.0);
        assert_eq!(v4.x(), 1.0);
        assert_eq!(v4.y(), 2.0);
        assert_eq!(v4.z(), 3.0);
        assert_eq!(v4.w(), 4.0);
    }

    #[test]
    fn vector3a_mapped_transforms_components() {
        let v = Vector3A::new(1.0, -2.0, 3.0);
        let mapped = v.mapped(|x| x * 2.0);
        assert_eq!(mapped.x(), 2.0);
        assert_eq!(mapped.y(), -4.0);
        assert_eq!(mapped.z(), 6.0);
    }

    #[test]
    fn vector3a_min_component_returns_smallest_element() {
        let v = Vector3A::new(3.7, 1.5, 2.1);
        assert_abs_diff_eq!(v.min_component(), 1.5, epsilon = EPSILON);
    }

    #[test]
    fn vector3a_unaligned_conversion_works() {
        let v3a = Vector3A::new(1.0, 2.0, 3.0);
        let v3 = v3a.unaligned();
        assert_eq!(v3.x(), 1.0);
        assert_eq!(v3.y(), 2.0);
        assert_eq!(v3.z(), 3.0);
    }

    // Vector3A tests for complete coverage
    #[test]
    fn vector3a_component_mutators_work() {
        let mut v = Vector3A::new(1.0, 2.0, 3.0);

        *v.x_mut() = 10.0;
        *v.y_mut() = 20.0;
        *v.z_mut() = 30.0;

        assert_eq!(v.x(), 10.0);
        assert_eq!(v.y(), 20.0);
        assert_eq!(v.z(), 30.0);
    }

    #[test]
    fn vector3a_norm_calculations_work() {
        let v = Vector3A::new(1.0, 2.0, 2.0);
        assert_abs_diff_eq!(v.norm(), 3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 9.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3a_normalized_gives_unit_vector() {
        let v = Vector3A::new(2.0, 0.0, 0.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3a_dot_product_works() {
        let v1 = Vector3A::new(1.0, 2.0, 3.0);
        let v2 = Vector3A::new(4.0, 5.0, 6.0);
        assert_abs_diff_eq!(v1.dot(&v2), 32.0, epsilon = EPSILON); // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn vector3a_component_operations_work() {
        let v1 = Vector3A::new(-1.0, 2.0, -3.0);
        let v2 = Vector3A::new(4.0, -5.0, 6.0);

        let abs_v = v1.component_abs();
        assert_eq!(abs_v.x(), 1.0);
        assert_eq!(abs_v.y(), 2.0);
        assert_eq!(abs_v.z(), 3.0);

        let mul_v = v1.component_mul(&v2);
        assert_eq!(mul_v.x(), -4.0);
        assert_eq!(mul_v.y(), -10.0);
        assert_eq!(mul_v.z(), -18.0);

        let min_v = v1.component_min(&v2);
        assert_eq!(min_v.x(), -1.0);
        assert_eq!(min_v.y(), -5.0);
        assert_eq!(min_v.z(), -3.0);

        let max_v = v1.component_max(&v2);
        assert_eq!(max_v.x(), 4.0);
        assert_eq!(max_v.y(), 2.0);
        assert_eq!(max_v.z(), 6.0);
    }

    #[test]
    fn vector3a_max_component_returns_largest_element() {
        let v = Vector3A::new(1.5, 3.7, 2.1);
        assert_abs_diff_eq!(v.max_component(), 3.7, epsilon = EPSILON);
    }

    #[test]
    fn vector3a_arithmetic_operations_work() {
        let v1 = Vector3A::new(1.0, 2.0, 3.0);
        let v2 = Vector3A::new(4.0, 5.0, 6.0);

        let add_result = &v1 + &v2;
        assert_eq!(add_result.x(), 5.0);
        assert_eq!(add_result.y(), 7.0);
        assert_eq!(add_result.z(), 9.0);

        let sub_result = &v1 - &v2;
        assert_eq!(sub_result.x(), -3.0);
        assert_eq!(sub_result.y(), -3.0);
        assert_eq!(sub_result.z(), -3.0);

        let mul_scalar = &v1 * 2.0;
        assert_eq!(mul_scalar.x(), 2.0);
        assert_eq!(mul_scalar.y(), 4.0);
        assert_eq!(mul_scalar.z(), 6.0);

        let scalar_mul = 3.0 * &v1;
        assert_eq!(scalar_mul.x(), 3.0);
        assert_eq!(scalar_mul.y(), 6.0);
        assert_eq!(scalar_mul.z(), 9.0);

        let div_scalar = &v1 / 2.0;
        assert_eq!(div_scalar.x(), 0.5);
        assert_eq!(div_scalar.y(), 1.0);
        assert_eq!(div_scalar.z(), 1.5);

        let neg_v = -&v1;
        assert_eq!(neg_v.x(), -1.0);
        assert_eq!(neg_v.y(), -2.0);
        assert_eq!(neg_v.z(), -3.0);
    }

    #[test]
    fn vector3a_assignment_operations_work() {
        let mut v1 = Vector3A::new(1.0, 2.0, 3.0);
        let v2 = Vector3A::new(4.0, 5.0, 6.0);

        v1 += &v2;
        assert_eq!(v1.x(), 5.0);
        assert_eq!(v1.y(), 7.0);
        assert_eq!(v1.z(), 9.0);

        v1 -= &v2;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);

        v1 *= 2.0;
        assert_eq!(v1.x(), 2.0);
        assert_eq!(v1.y(), 4.0);
        assert_eq!(v1.z(), 6.0);

        v1 /= 2.0;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);
    }

    #[test]
    fn vector3a_indexing_works() {
        let mut v = Vector3A::new(1.0, 2.0, 3.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);

        v[0] = 10.0;
        v[1] = 20.0;
        v[2] = 30.0;
        assert_eq!(v[0], 10.0);
        assert_eq!(v[1], 20.0);
        assert_eq!(v[2], 30.0);
    }

    #[test]
    fn vector3a_array_conversion_works() {
        let arr: [f32; 3] = [1.0, 2.0, 3.0];
        let v = Vector3A::from(arr);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);

        let converted_back: [f32; 3] = v.into();
        assert_eq!(converted_back, [1.0, 2.0, 3.0]);
    }

    // Vector4A tests for complete coverage
    #[test]
    fn vector4a_zeros_gives_zero_vector() {
        let v = Vector4A::zeros();
        assert_eq!(v.x(), 0.0);
        assert_eq!(v.y(), 0.0);
        assert_eq!(v.z(), 0.0);
        assert_eq!(v.w(), 0.0);
    }

    #[test]
    fn vector4a_same_creates_vector_with_repeated_value() {
        let v = Vector4A::same(1.5);
        assert_eq!(v.x(), 1.5);
        assert_eq!(v.y(), 1.5);
        assert_eq!(v.z(), 1.5);
        assert_eq!(v.w(), 1.5);
    }

    #[test]
    fn vector4a_unit_vectors_work() {
        let unit_x = Vector4A::unit_x();
        assert_eq!(unit_x.x(), 1.0);
        assert_eq!(unit_x.y(), 0.0);
        assert_eq!(unit_x.z(), 0.0);
        assert_eq!(unit_x.w(), 0.0);

        let unit_y = Vector4A::unit_y();
        assert_eq!(unit_y.x(), 0.0);
        assert_eq!(unit_y.y(), 1.0);
        assert_eq!(unit_y.z(), 0.0);
        assert_eq!(unit_y.w(), 0.0);

        let unit_z = Vector4A::unit_z();
        assert_eq!(unit_z.x(), 0.0);
        assert_eq!(unit_z.y(), 0.0);
        assert_eq!(unit_z.z(), 1.0);
        assert_eq!(unit_z.w(), 0.0);

        let unit_w = Vector4A::unit_w();
        assert_eq!(unit_w.x(), 0.0);
        assert_eq!(unit_w.y(), 0.0);
        assert_eq!(unit_w.z(), 0.0);
        assert_eq!(unit_w.w(), 1.0);
    }

    #[test]
    fn vector4a_mapped_transforms_components() {
        let v = Vector4A::new(1.0, -2.0, 3.0, -4.0);
        let mapped = v.mapped(|x| x * 2.0);
        assert_eq!(mapped.x(), 2.0);
        assert_eq!(mapped.y(), -4.0);
        assert_eq!(mapped.z(), 6.0);
        assert_eq!(mapped.w(), -8.0);
    }

    #[test]
    fn vector4a_min_component_returns_smallest_element() {
        let v = Vector4A::new(3.7, 1.5, 2.1, 0.8);
        assert_abs_diff_eq!(v.min_component(), 0.8, epsilon = EPSILON);
    }

    #[test]
    fn vector4a_unaligned_conversion_works() {
        let v4a = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let v4 = v4a.unaligned();
        assert_eq!(v4.x(), 1.0);
        assert_eq!(v4.y(), 2.0);
        assert_eq!(v4.z(), 3.0);
        assert_eq!(v4.w(), 4.0);
    }

    #[test]
    fn vector4a_norm_calculations_work() {
        let v = Vector4A::new(1.0, 2.0, 2.0, 0.0);
        assert_abs_diff_eq!(v.norm(), 3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 9.0, epsilon = EPSILON);
    }

    #[test]
    fn vector4a_normalized_gives_unit_vector() {
        let v = Vector4A::new(2.0, 0.0, 0.0, 0.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.z(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.w(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector4a_dot_product_works() {
        let v1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);
        assert_abs_diff_eq!(v1.dot(&v2), 70.0, epsilon = EPSILON); // 1*5 + 2*6 + 3*7 + 4*8 = 70
    }

    #[test]
    fn vector4a_component_operations_work() {
        let v1 = Vector4A::new(-1.0, 2.0, -3.0, 4.0);
        let v2 = Vector4A::new(5.0, -6.0, 7.0, -8.0);

        let abs_v = v1.component_abs();
        assert_eq!(abs_v.x(), 1.0);
        assert_eq!(abs_v.y(), 2.0);
        assert_eq!(abs_v.z(), 3.0);
        assert_eq!(abs_v.w(), 4.0);

        let mul_v = v1.component_mul(&v2);
        assert_eq!(mul_v.x(), -5.0);
        assert_eq!(mul_v.y(), -12.0);
        assert_eq!(mul_v.z(), -21.0);
        assert_eq!(mul_v.w(), -32.0);

        let min_v = v1.component_min(&v2);
        assert_eq!(min_v.x(), -1.0);
        assert_eq!(min_v.y(), -6.0);
        assert_eq!(min_v.z(), -3.0);
        assert_eq!(min_v.w(), -8.0);

        let max_v = v1.component_max(&v2);
        assert_eq!(max_v.x(), 5.0);
        assert_eq!(max_v.y(), 2.0);
        assert_eq!(max_v.z(), 7.0);
        assert_eq!(max_v.w(), 4.0);
    }

    #[test]
    fn vector4a_max_component_returns_largest_element() {
        let v = Vector4A::new(1.5, 3.7, 2.1, 0.8);
        assert_abs_diff_eq!(v.max_component(), 3.7, epsilon = EPSILON);
    }

    #[test]
    fn vector4a_arithmetic_operations_work() {
        let v1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);

        let add_result = &v1 + &v2;
        assert_eq!(add_result.x(), 6.0);
        assert_eq!(add_result.y(), 8.0);
        assert_eq!(add_result.z(), 10.0);
        assert_eq!(add_result.w(), 12.0);

        let sub_result = &v1 - &v2;
        assert_eq!(sub_result.x(), -4.0);
        assert_eq!(sub_result.y(), -4.0);
        assert_eq!(sub_result.z(), -4.0);
        assert_eq!(sub_result.w(), -4.0);

        let mul_scalar = &v1 * 2.0;
        assert_eq!(mul_scalar.x(), 2.0);
        assert_eq!(mul_scalar.y(), 4.0);
        assert_eq!(mul_scalar.z(), 6.0);
        assert_eq!(mul_scalar.w(), 8.0);

        let scalar_mul = 3.0 * &v1;
        assert_eq!(scalar_mul.x(), 3.0);
        assert_eq!(scalar_mul.y(), 6.0);
        assert_eq!(scalar_mul.z(), 9.0);
        assert_eq!(scalar_mul.w(), 12.0);

        let div_scalar = &v1 / 2.0;
        assert_eq!(div_scalar.x(), 0.5);
        assert_eq!(div_scalar.y(), 1.0);
        assert_eq!(div_scalar.z(), 1.5);
        assert_eq!(div_scalar.w(), 2.0);

        let neg_v = -&v1;
        assert_eq!(neg_v.x(), -1.0);
        assert_eq!(neg_v.y(), -2.0);
        assert_eq!(neg_v.z(), -3.0);
        assert_eq!(neg_v.w(), -4.0);
    }

    #[test]
    fn vector4a_assignment_operations_work() {
        let mut v1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);

        v1 += &v2;
        assert_eq!(v1.x(), 6.0);
        assert_eq!(v1.y(), 8.0);
        assert_eq!(v1.z(), 10.0);
        assert_eq!(v1.w(), 12.0);

        v1 -= &v2;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);
        assert_eq!(v1.w(), 4.0);

        v1 *= 2.0;
        assert_eq!(v1.x(), 2.0);
        assert_eq!(v1.y(), 4.0);
        assert_eq!(v1.z(), 6.0);
        assert_eq!(v1.w(), 8.0);

        v1 /= 2.0;
        assert_eq!(v1.x(), 1.0);
        assert_eq!(v1.y(), 2.0);
        assert_eq!(v1.z(), 3.0);
        assert_eq!(v1.w(), 4.0);
    }

    #[test]
    fn vector4a_indexing_works() {
        let mut v = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);
        assert_eq!(v[3], 4.0);

        v[0] = 10.0;
        v[1] = 20.0;
        v[2] = 30.0;
        v[3] = 40.0;
        assert_eq!(v[0], 10.0);
        assert_eq!(v[1], 20.0);
        assert_eq!(v[2], 30.0);
        assert_eq!(v[3], 40.0);
    }

    #[test]
    fn vector4a_array_conversion_works() {
        let arr: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        let v = Vector4A::from(arr);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.w(), 4.0);

        let converted_back: [f32; 4] = v.into();
        assert_eq!(converted_back, [1.0, 2.0, 3.0, 4.0]);
    }

    // Cross-type conversion tests
    #[test]
    fn vector3_to_vector3a_conversion_works() {
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let v3a = v3.aligned();
        assert_eq!(v3a.x(), 1.0);
        assert_eq!(v3a.y(), 2.0);
        assert_eq!(v3a.z(), 3.0);
    }

    #[test]
    fn vector3a_to_vector3_conversion_works() {
        let v3a = Vector3A::new(1.0, 2.0, 3.0);
        let v3 = v3a.unaligned();
        assert_eq!(v3.x(), 1.0);
        assert_eq!(v3.y(), 2.0);
        assert_eq!(v3.z(), 3.0);
    }

    #[test]
    fn vector4_to_vector4a_conversion_works() {
        let v4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v4a = v4.aligned();
        assert_eq!(v4a.x(), 1.0);
        assert_eq!(v4a.y(), 2.0);
        assert_eq!(v4a.z(), 3.0);
        assert_eq!(v4a.w(), 4.0);
    }

    #[test]
    fn vector4a_to_vector4_conversion_works() {
        let v4a = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let v4 = v4a.unaligned();
        assert_eq!(v4.x(), 1.0);
        assert_eq!(v4.y(), 2.0);
        assert_eq!(v4.z(), 3.0);
        assert_eq!(v4.w(), 4.0);
    }

    #[test]
    fn unitvector3_to_unitvector3a_conversion_works() {
        let unit3 = UnitVector3::unit_x();
        let unit3a = unit3.aligned();
        assert_eq!(unit3a.x(), 1.0);
        assert_eq!(unit3a.y(), 0.0);
        assert_eq!(unit3a.z(), 0.0);
    }

    #[test]
    fn unitvector3a_to_unitvector3_conversion_works() {
        let unit3a = UnitVector3A::unit_x();
        let unit3 = unit3a.unaligned();
        assert_eq!(unit3.x(), 1.0);
        assert_eq!(unit3.y(), 0.0);
        assert_eq!(unit3.z(), 0.0);
    }

    // Edge cases and boundary conditions
    #[test]
    #[should_panic]
    fn vector2_indexing_panics_on_out_of_bounds() {
        let v = Vector2::new(1.0, 2.0);
        let _ = v[2]; // Should panic
    }

    #[test]
    #[should_panic]
    fn vector3_indexing_panics_on_out_of_bounds() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        let _ = v[3]; // Should panic
    }

    #[test]
    #[should_panic]
    fn vector3a_indexing_panics_on_out_of_bounds() {
        let v = Vector3A::new(1.0, 2.0, 3.0);
        let _ = v[3]; // Should panic
    }

    #[test]
    #[should_panic]
    fn vector4_indexing_panics_on_out_of_bounds() {
        let v = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let _ = v[4]; // Should panic
    }

    #[test]
    #[should_panic]
    fn vector4a_indexing_panics_on_out_of_bounds() {
        let v = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let _ = v[4]; // Should panic
    }

    #[test]
    fn vector2_normalized_zero_vector_returns_nan() {
        let zero = Vector2::zeros();
        let normalized = zero.normalized();
        assert!(normalized.x().is_nan());
        assert!(normalized.y().is_nan());
    }

    #[test]
    fn vector3a_normalized_zero_vector_returns_nan() {
        let zero = Vector3A::zeros();
        let normalized = zero.normalized();
        assert!(normalized.x().is_nan());
        assert!(normalized.y().is_nan());
        assert!(normalized.z().is_nan());
    }

    #[test]
    fn vector4a_normalized_zero_vector_returns_nan() {
        let zero = Vector4A::zeros();
        let normalized = zero.normalized();
        assert!(normalized.x().is_nan());
        assert!(normalized.y().is_nan());
        assert!(normalized.z().is_nan());
        assert!(normalized.w().is_nan());
    }

    #[test]
    fn unitvector3a_from_zero_vector_returns_nan() {
        let zero = Vector3A::zeros();
        let unit = UnitVector3A::normalized_from(zero);
        assert!(unit.norm().is_nan());
    }

    #[test]
    fn vector_operations_with_different_reference_combinations_work() {
        // Test Vector2
        let v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);

        // Test all combinations of reference/owned for binary operations
        let _result1 = &v1 + &v2; // ref + ref
        let _result2 = &v1 + v2; // ref + owned
        let _result3 = v1 + &v2; // owned + ref
        let _result4 = v1 + v2; // owned + owned

        // Recreate vectors since they were moved
        let v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);
        let _result5 = 2.0 * &v1; // scalar * ref
        let _result6 = 2.0 * v1; // scalar * owned
        let _result7 = &v2 * 2.0; // ref * scalar
        let _result8 = v2 * 2.0; // owned * scalar

        // Test Vector3
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        let _result = &v1 + &v2;
        let _result = v1 - v2;
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let _result = 2.0 * v1;

        // Test Vector3A
        let v1 = Vector3A::new(1.0, 2.0, 3.0);
        let v2 = Vector3A::new(4.0, 5.0, 6.0);
        let _result = &v1 + &v2;
        let _result = v1 - v2;
        let v1 = Vector3A::new(1.0, 2.0, 3.0);
        let _result = &v1 * 2.0;

        // Test Vector4
        let v1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        let _result = &v1 + &v2;
        let _result = v1 - v2;
        let v1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let _result = v1 / 2.0;

        // Test Vector4A
        let v1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4A::new(5.0, 6.0, 7.0, 8.0);
        let _result = &v1 + &v2;
        let _result = v1 - v2;
        let v1 = Vector4A::new(1.0, 2.0, 3.0, 4.0);
        let _result = 3.0 * &v1;
    }

    #[test]
    fn vector_arithmetic_maintains_precision() {
        let v = Vector3A::new(0.1, 0.2, 0.3);
        let doubled = &v * 2.0;
        let halved = &doubled / 2.0;

        assert_abs_diff_eq!(halved.x(), v.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.y(), v.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.z(), v.z(), epsilon = EPSILON);
    }

    #[test]
    fn unitvector3a_maintains_unit_length_through_operations() {
        let unit = UnitVector3A::normalized_from(Vector3A::new(1.0, 2.0, 3.0));
        let scaled_back = (&unit * 5.0) / 5.0;

        assert_abs_diff_eq!(scaled_back.norm(), 1.0, epsilon = EPSILON);
    }
}
