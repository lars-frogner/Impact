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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Vector2 {
    inner: glam::Vec2,
}

/// A 3-dimensional vector.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an extra 4 bytes in size and 16-byte alignment.
/// For cache-friendly storage, prefer the compact 4-byte aligned [`Vector3C`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Vector3 {
    inner: glam::Vec3A,
}

/// A 3-dimensional vector. This is the "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Vector3`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 3]", from = "[f32; 3]")
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Vector3C {
    x: f32,
    y: f32,
    z: f32,
}

/// A 3-dimensional vector of unit length.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an extra 4 bytes in size and 16-byte alignment.
/// For cache-friendly storage, prefer the compact 4-byte aligned
/// [`UnitVector3C`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, PartialEq, Zeroable, Pod)]
pub struct UnitVector3 {
    inner: glam::Vec3A,
}

/// A 3-dimensional vector of unit length. This is the "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`UnitVector3`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 3]", from = "[f32; 3]")
)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct UnitVector3C {
    x: f32,
    y: f32,
    z: f32,
}

/// A 4-dimensional vector.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an alignment of 16 bytes. For padding-free
/// storage together with smaller types, prefer the 4-byte aligned [`Vector4C`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Vector4 {
    inner: glam::Vec4,
}

/// A 4-dimensional vector. This is the "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// padding-free storage when combined with smaller types. For computations,
/// prefer the SIMD-friendly 16-byte aligned [`Vector4`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 4]", from = "[f32; 4]")
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Vector4C {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
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
    pub const fn extended(&self, z: f32) -> Vector3C {
        Vector3C::new(self.x(), self.y(), z)
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
    pub fn extended(&self, w: f32) -> Vector4 {
        Vector4::new(self.x(), self.y(), self.z(), w)
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

    /// Computes the product of the three vector components.
    #[inline]
    pub fn component_product(&self) -> f32 {
        self.inner.element_product()
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

    /// Converts the vector to the 4-byte aligned cache-friendly [`Vector3C`].
    #[inline]
    pub fn compact(&self) -> Vector3C {
        Vector3C::new(self.x(), self.y(), self.z())
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
    Vector3::wrap(a.inner.add(b.inner))
});

impl_binop!(Sub, sub, Vector3, Vector3, Vector3, |a, b| {
    Vector3::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Vector3, f32, Vector3, |a, b| {
    Vector3::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Vector3, Vector3, |a, b| { b.mul(a) });

impl_binop!(Div, div, Vector3, f32, Vector3, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Vector3, Vector3, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Vector3, Vector3, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Vector3, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector3, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector3, Vector3, |val| {
    Vector3::wrap(val.inner.neg())
});

impl Index<usize> for Vector3 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector3 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

impl_abs_diff_eq!(Vector3, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Vector3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vector3")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .finish()
    }
}

impl Vector3C {
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
    pub const fn extended(&self, w: f32) -> Vector4C {
        Vector4C::new(self.x(), self.y(), self.z(), w)
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

    /// Computes the product of the three vector components.
    #[inline]
    pub fn component_product(&self) -> f32 {
        self.x * self.y * self.z
    }

    /// Computes the dot product of this vector with another.
    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Computes the cross product of this vector with another.
    #[inline]
    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Returns a vector with the absolute value of each component.
    #[inline]
    pub fn component_abs(&self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    /// Multiplies each component by the corresponding component in another
    /// vector.
    #[inline]
    pub fn component_mul(&self, other: &Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }

    /// Returns a vector where each component is the minimum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_min(&self, other: &Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    /// Returns a vector where each component is the maximum of the
    /// corresponding component in this and another vector.
    #[inline]
    pub fn component_max(&self, other: &Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    /// Returns a vector with the given closure applied to each component.
    #[inline]
    pub fn mapped(&self, mut f: impl FnMut(f32) -> f32) -> Self {
        Self::new(f(self.x()), f(self.y()), f(self.z()))
    }

    /// Returns the smallest component in the vector.
    #[inline]
    pub fn min_component(&self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    /// Returns the largest component in the vector.
    #[inline]
    pub fn max_component(&self) -> f32 {
        self.x.max(self.y).max(self.z)
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly [`Vector3`].
    #[inline]
    pub fn aligned(&self) -> Vector3 {
        Vector3::new(self.x(), self.y(), self.z())
    }

    #[inline]
    pub(crate) const fn from_glam(vector: glam::Vec3) -> Self {
        Self::new(vector.x, vector.y, vector.z)
    }
}

impl From<[f32; 3]> for Vector3C {
    #[inline]
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Vector3C> for [f32; 3] {
    #[inline]
    fn from(vector: Vector3C) -> Self {
        [vector.x(), vector.y(), vector.z()]
    }
}

impl_binop!(Add, add, Vector3C, Vector3C, Vector3C, |a, b| {
    Vector3C::new(a.x + b.x, a.y + b.y, a.z + b.z)
});

impl_binop!(Sub, sub, Vector3C, Vector3C, Vector3C, |a, b| {
    Vector3C::new(a.x - b.x, a.y - b.y, a.z - b.z)
});

impl_binop!(Mul, mul, Vector3C, f32, Vector3C, |a, b| {
    Vector3C::new(a.x * b, a.y * b, a.z * b)
});

impl_binop!(Mul, mul, f32, Vector3C, Vector3C, |a, b| { b.mul(a) });

impl_binop!(Div, div, Vector3C, f32, Vector3C, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, Vector3C, Vector3C, |a, b| {
    a.x += b.x;
    a.y += b.y;
    a.z += b.z;
});

impl_binop_assign!(SubAssign, sub_assign, Vector3C, Vector3C, |a, b| {
    a.x -= b.x;
    a.y -= b.y;
    a.z -= b.z;
});

impl_binop_assign!(MulAssign, mul_assign, Vector3C, f32, |a, b| {
    a.x *= b;
    a.y *= b;
    a.z *= b;
});

impl_binop_assign!(DivAssign, div_assign, Vector3C, f32, |a, b| {
    a.mul_assign(b.recip());
});

impl_unary_op!(Neg, neg, Vector3C, Vector3C, |val| {
    Vector3C::new(-val.x, -val.y, -val.z)
});

impl Index<usize> for Vector3C {
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

impl IndexMut<usize> for Vector3C {
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

impl_abs_diff_eq!(Vector3C, |a, b, epsilon| {
    a.x.abs_diff_eq(&b.x, epsilon)
        && a.y.abs_diff_eq(&b.y, epsilon)
        && a.z.abs_diff_eq(&b.z, epsilon)
});

impl_relative_eq!(Vector3C, |a, b, epsilon, max_relative| {
    a.x.relative_eq(&b.x, epsilon, max_relative)
        && a.y.relative_eq(&b.y, epsilon, max_relative)
        && a.z.relative_eq(&b.z, epsilon, max_relative)
});

impl UnitVector3 {
    /// Creates a vector with the given components. The vector is assumed to be
    /// normalized.
    #[inline]
    pub const fn new_unchecked(x: f32, y: f32, z: f32) -> Self {
        Self::wrap(glam::Vec3A::new(x, y, z))
    }

    /// Converts the given vector to a unit vector, assuming it is already
    /// normalized.
    #[inline]
    pub const fn unchecked_from(vector: Vector3) -> Self {
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
    pub fn normalized_from(vector: Vector3) -> Self {
        Self::wrap(vector.unwrap().normalize())
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
        let (inner, norm) = vector.unwrap().normalize_and_length();
        (Self::wrap(inner), norm)
    }

    /// Creates a unit vector by normalizing the given vector if its norm
    /// exceeds the given threshold, and returns both the vector and the norm.
    /// Returns [`None`] if the norm does not exceed the threshold.
    #[inline]
    pub fn normalized_from_and_norm_if_above(
        vector: Vector3,
        min_norm: f32,
    ) -> Option<(Self, f32)> {
        let v = vector.unwrap();
        let norm_squared = v.length_squared();
        (norm_squared > min_norm.powi(2)).then(|| {
            let norm = norm_squared.sqrt();
            (Self::wrap(v / norm), norm)
        })
    }

    /// Creates a unit vector that is orthogonal to the given unit vector. The
    /// choice of vector is left to the implementation.
    #[inline]
    pub fn orthogonal_to(unit_vector: &Self) -> Self {
        Self::wrap(unit_vector.inner.any_orthonormal_vector())
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

    /// This unit vector as a [`Vector3`].
    #[inline]
    pub fn as_vector(&self) -> &Vector3 {
        self // deref
    }

    /// Converts the vector to the 4-byte aligned cache-friendly
    /// [`UnitVector3C`].
    #[inline]
    pub fn compact(&self) -> UnitVector3C {
        UnitVector3C::new_unchecked(self.x(), self.y(), self.z())
    }

    #[inline]
    pub(crate) const fn wrap(inner: glam::Vec3A) -> Self {
        Self { inner }
    }
}

impl Deref for UnitVector3 {
    type Target = Vector3;

    #[inline]
    fn deref(&self) -> &Self::Target {
        bytemuck::cast_ref(self)
    }
}

impl_binop!(Mul, mul, UnitVector3, f32, Vector3, |a, b| {
    Vector3::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, UnitVector3, Vector3, |a, b| { b.mul(*a) });

impl_binop!(Div, div, UnitVector3, f32, Vector3, |a, b| {
    a.mul(b.recip())
});

impl_unary_op!(Neg, neg, UnitVector3, UnitVector3, |val| {
    UnitVector3::wrap(val.inner.neg())
});

impl Index<usize> for UnitVector3 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl_abs_diff_eq!(UnitVector3, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(UnitVector3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for UnitVector3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitVector3")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .finish()
    }
}

impl UnitVector3C {
    /// Creates a vector with the given components. The vector is assumed to be
    /// normalized.
    #[inline]
    pub const fn new_unchecked(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Converts the given vector to a unit vector, assuming it is already
    /// normalized.
    #[inline]
    pub const fn unchecked_from(vector: Vector3C) -> Self {
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
    pub fn normalized_from(vector: Vector3C) -> Self {
        Self::unchecked_from(vector.normalized())
    }

    /// Creates a unit vector by normalizing the given vector if its norm
    /// exceeds the given threshold. Otherwise, returns [`None`].
    #[inline]
    pub fn normalized_from_if_above(vector: Vector3C, min_norm: f32) -> Option<Self> {
        Self::normalized_from_and_norm_if_above(vector, min_norm).map(|(v, _norm)| v)
    }

    /// Creates a unit vector by normalizing the given vector, and returns both
    /// the vector and the norm. If the norm is zero, the vector will be
    /// non-finite.
    #[inline]
    pub fn normalized_from_and_norm(vector: Vector3C) -> (Self, f32) {
        let norm = vector.norm();
        (Self::unchecked_from(vector / norm), norm)
    }

    /// Creates a unit vector by normalizing the given vector if its norm
    /// exceeds the given threshold, and returns both the vector and the norm.
    /// Returns [`None`] if the norm does not exceed the threshold.
    #[inline]
    pub fn normalized_from_and_norm_if_above(
        vector: Vector3C,
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

    /// This unit vector as a [`Vector3C`].
    #[inline]
    pub fn as_vector(&self) -> &Vector3C {
        self // deref
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly
    /// [`UnitVector3`].
    #[inline]
    pub fn aligned(&self) -> UnitVector3 {
        UnitVector3::new_unchecked(self.x(), self.y(), self.z())
    }
}

impl Deref for UnitVector3C {
    type Target = Vector3C;

    #[inline]
    fn deref(&self) -> &Self::Target {
        bytemuck::cast_ref(self)
    }
}

impl From<UnitVector3C> for [f32; 3] {
    fn from(vector: UnitVector3C) -> Self {
        [vector.x(), vector.y(), vector.z()]
    }
}

impl From<[f32; 3]> for UnitVector3C {
    fn from(vector: [f32; 3]) -> Self {
        Self::normalized_from(vector.into())
    }
}

impl_binop!(Mul, mul, UnitVector3C, f32, Vector3C, |a, b| {
    Vector3C::new(a.x * b, a.y * b, a.z * b)
});

impl_binop!(Mul, mul, f32, UnitVector3C, Vector3C, |a, b| { b.mul(*a) });

impl_binop!(Div, div, UnitVector3C, f32, Vector3C, |a, b| {
    a.mul(b.recip())
});

impl_unary_op!(Neg, neg, UnitVector3C, UnitVector3C, |val| {
    UnitVector3C::new_unchecked(-val.x, -val.y, -val.z)
});

impl Index<usize> for UnitVector3C {
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

impl_abs_diff_eq!(UnitVector3C, |a, b, epsilon| {
    a.x.abs_diff_eq(&b.x, epsilon)
        && a.y.abs_diff_eq(&b.y, epsilon)
        && a.z.abs_diff_eq(&b.z, epsilon)
});

impl_relative_eq!(UnitVector3C, |a, b, epsilon, max_relative| {
    a.x.relative_eq(&b.x, epsilon, max_relative)
        && a.y.relative_eq(&b.y, epsilon, max_relative)
        && a.z.relative_eq(&b.z, epsilon, max_relative)
});

impl Vector4 {
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
    pub fn xyz(&self) -> Vector3 {
        Vector3::new(self.x(), self.y(), self.z())
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

    /// Converts the vector to the 4-byte aligned cache-friendly [`Vector4C`].
    #[inline]
    pub fn compact(&self) -> Vector4C {
        Vector4C::new(self.x(), self.y(), self.z(), self.w())
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
    Vector4::wrap(a.inner.add(b.inner))
});

impl_binop!(Sub, sub, Vector4, Vector4, Vector4, |a, b| {
    Vector4::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Vector4, f32, Vector4, |a, b| {
    Vector4::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Vector4, Vector4, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Vector4, f32, Vector4, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Vector4, Vector4, |a, b| {
    a.inner.add_assign(b.inner);
});

impl_binop_assign!(SubAssign, sub_assign, Vector4, Vector4, |a, b| {
    a.inner.sub_assign(b.inner);
});

impl_binop_assign!(MulAssign, mul_assign, Vector4, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector4, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector4, Vector4, |val| {
    Vector4::wrap(val.inner.neg())
});

impl Index<usize> for Vector4 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector4 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

impl_abs_diff_eq!(Vector4, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Vector4, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl fmt::Debug for Vector4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vector4")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .field("w", &self.inner.w)
            .finish()
    }
}

impl Vector4C {
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
    pub const fn xyz(&self) -> Vector3C {
        Vector3C::new(self.x(), self.y(), self.z())
    }

    /// Converts the vector to the 16-byte aligned SIMD-friendly [`Vector4`].
    #[inline]
    pub fn aligned(&self) -> Vector4 {
        Vector4::new(self.x(), self.y(), self.z(), self.w())
    }
}

impl From<[f32; 4]> for Vector4C {
    #[inline]
    fn from([x, y, z, w]: [f32; 4]) -> Self {
        Self::new(x, y, z, w)
    }
}

impl From<Vector4C> for [f32; 4] {
    #[inline]
    fn from(vector: Vector4C) -> Self {
        [vector.x(), vector.y(), vector.z(), vector.w()]
    }
}

impl_binop!(Add, add, Vector4C, Vector4C, Vector4C, |a, b| {
    Vector4C::new(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
});

impl_binop!(Sub, sub, Vector4C, Vector4C, Vector4C, |a, b| {
    Vector4C::new(a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w)
});

impl_binop!(Mul, mul, Vector4C, f32, Vector4C, |a, b| {
    Vector4C::new(a.x * b, a.y * b, a.z * b, a.w * b)
});

impl_binop!(Mul, mul, f32, Vector4C, Vector4C, |a, b| { b.mul(a) });

impl_binop!(Div, div, Vector4C, f32, Vector4C, |a, b| {
    a.mul(b.recip())
});

impl_binop_assign!(AddAssign, add_assign, Vector4C, Vector4C, |a, b| {
    a.x += b.x;
    a.y += b.y;
    a.z += b.z;
    a.w += b.w;
});

impl_binop_assign!(SubAssign, sub_assign, Vector4C, Vector4C, |a, b| {
    a.x -= b.x;
    a.y -= b.y;
    a.z -= b.z;
    a.w -= b.w;
});

impl_binop_assign!(MulAssign, mul_assign, Vector4C, f32, |a, b| {
    a.x *= b;
    a.y *= b;
    a.z *= b;
    a.w *= b;
});

impl_binop_assign!(DivAssign, div_assign, Vector4C, f32, |a, b| {
    a.mul_assign(b.recip());
});

impl_unary_op!(Neg, neg, Vector4C, Vector4C, |val| {
    Vector4C::new(-val.x, -val.y, -val.z, -val.w)
});

impl Index<usize> for Vector4C {
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

impl IndexMut<usize> for Vector4C {
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

impl_abs_diff_eq!(Vector4C, |a, b, epsilon| {
    a.x.abs_diff_eq(&b.x, epsilon)
        && a.y.abs_diff_eq(&b.y, epsilon)
        && a.z.abs_diff_eq(&b.z, epsilon)
        && a.w.abs_diff_eq(&b.w, epsilon)
});

impl_relative_eq!(Vector4C, |a, b, epsilon, max_relative| {
    a.x.relative_eq(&b.x, epsilon, max_relative)
        && a.y.relative_eq(&b.y, epsilon, max_relative)
        && a.z.relative_eq(&b.z, epsilon, max_relative)
        && a.w.relative_eq(&b.w, epsilon, max_relative)
});

impl_roc_for_library_provided_primitives! {
//  Type            Pkg   Parents  Module       Roc name     Postfix  Precision
    Vector2      => core, None,    Vector2,     Vector2,     None,    PrecisionIrrelevant,
    Vector3C     => core, None,    Vector3,     Vector3,     None,    PrecisionIrrelevant,
    UnitVector3C => core, None,    UnitVector3, UnitVector3, None,    PrecisionIrrelevant,
}

#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for UnitVector3 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let x = arbitrary_norm_f32(u)?;
        let y = arbitrary_norm_f32(u)?;
        let z = arbitrary_norm_f32(u)?;
        Ok(
            if approx::abs_diff_eq!(x, 0.0)
                && approx::abs_diff_eq!(y, 0.0)
                && approx::abs_diff_eq!(z, 0.0)
            {
                Self::unit_y()
            } else {
                Self::normalized_from(Vector3::new(x, y, z))
            },
        )
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        let size = 3 * std::mem::size_of::<i32>();
        (size, Some(size))
    }
}

#[cfg(feature = "arbitrary")]
impl arbitrary::Arbitrary<'_> for UnitVector3C {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let x = arbitrary_norm_f32(u)?;
        let y = arbitrary_norm_f32(u)?;
        let z = arbitrary_norm_f32(u)?;
        Ok(
            if approx::abs_diff_eq!(x, 0.0)
                && approx::abs_diff_eq!(y, 0.0)
                && approx::abs_diff_eq!(z, 0.0)
            {
                Self::unit_y()
            } else {
                Self::normalized_from(Vector3C::new(x, y, z))
            },
        )
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        let size = 3 * std::mem::size_of::<i32>();
        (size, Some(size))
    }
}

#[cfg(feature = "arbitrary")]
fn arbitrary_norm_f32(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<f32> {
    Ok((f64::from(u.int_in_range(0..=1000000)?) / 1000000.0) as f32)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use approx::assert_abs_diff_eq;

    const EPSILON: f32 = 1e-6;

    // === Vector2 Tests ===

    #[test]
    fn computing_vector2_norm_works() {
        let v = Vector2::new(3.0, 4.0);
        assert_abs_diff_eq!(v.norm(), 5.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 25.0, epsilon = EPSILON);
    }

    #[test]
    fn normalizing_vector2_gives_unit_vector() {
        let v = Vector2::new(3.0, 4.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized, Vector2::new(0.6, 0.8), epsilon = EPSILON);
    }

    #[test]
    fn normalizing_zero_vector2_gives_nan() {
        let normalized = Vector2::zeros().normalized();
        assert!(normalized.x().is_nan() && normalized.y().is_nan());
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

        assert_eq!(v1.component_abs(), Vector2::new(1.0, 2.0));
        assert_eq!(v1.component_mul(&v2), Vector2::new(-3.0, -8.0));
        assert_eq!(v1.component_min(&v2), Vector2::new(-1.0, -4.0));
        assert_eq!(v1.component_max(&v2), Vector2::new(3.0, 2.0));
        assert_abs_diff_eq!(v1.min_component(), -1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v1.max_component(), 2.0, epsilon = EPSILON);
    }

    #[test]
    fn extending_vector2_to_vector3p_works() {
        let v2 = Vector2::new(1.0, 2.0);
        let v3 = v2.extended(3.0);
        assert_eq!(v3, Vector3C::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn mapping_vector2_components_works() {
        let v = Vector2::new(1.0, -2.0);
        let mapped = v.mapped(|x| x * 2.0);
        assert_eq!(mapped, Vector2::new(2.0, -4.0));
    }

    #[test]
    fn vector2_arithmetic_operations_work() {
        let v1 = Vector2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);

        assert_eq!(&v1 + &v2, Vector2::new(4.0, 6.0));
        assert_eq!(&v1 - &v2, Vector2::new(-2.0, -2.0));
        assert_eq!(&v1 * 2.0, Vector2::new(2.0, 4.0));
        assert_eq!(3.0 * &v1, Vector2::new(3.0, 6.0));
        assert_eq!(&v1 / 2.0, Vector2::new(0.5, 1.0));
        assert_eq!(-&v1, Vector2::new(-1.0, -2.0));
    }

    #[test]
    fn vector2_indexing_works() {
        let mut v = Vector2::new(1.0, 2.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);

        v[0] = 10.0;
        v[1] = 20.0;
        assert_eq!(v, Vector2::new(10.0, 20.0));
    }

    #[test]
    #[should_panic]
    fn indexing_vector2_out_of_bounds_panics() {
        let v = Vector2::new(1.0, 2.0);
        let _ = v[2];
    }

    // === Vector3 Tests (SIMD-aligned) ===

    #[test]
    fn computing_vector3_norm_works() {
        let v = Vector3::new(1.0, 2.0, 2.0);
        assert_abs_diff_eq!(v.norm(), 3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 9.0, epsilon = EPSILON);
    }

    #[test]
    fn normalizing_vector3_gives_unit_vector() {
        let v = Vector3::new(2.0, 0.0, 0.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized, Vector3::new(1.0, 0.0, 0.0), epsilon = EPSILON);
    }

    #[test]
    fn normalizing_zero_vector3_gives_nan() {
        let normalized = Vector3::zeros().normalized();
        assert!(normalized.x().is_nan());
    }

    #[test]
    fn vector3_dot_product_works() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        assert_abs_diff_eq!(v1.dot(&v2), 32.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3_cross_product_works() {
        let v1 = Vector3::new(1.0, 0.0, 0.0);
        let v2 = Vector3::new(0.0, 1.0, 0.0);
        let cross = v1.cross(&v2);
        assert_abs_diff_eq!(cross, Vector3::new(0.0, 0.0, 1.0), epsilon = EPSILON);
    }

    #[test]
    fn vector3_cross_product_is_perpendicular() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        let cross = v1.cross(&v2);

        assert_abs_diff_eq!(cross.dot(&v1), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.dot(&v2), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3_cross_product_is_anticommutative() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        let cross1 = v1.cross(&v2);
        let cross2 = v2.cross(&v1);

        assert_abs_diff_eq!(cross1, -&cross2, epsilon = EPSILON);
    }

    #[test]
    fn vector3_cross_product_of_parallel_vectors_is_zero() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(2.0, 4.0, 6.0);
        let cross = v1.cross(&v2);

        assert_abs_diff_eq!(cross, Vector3::zeros(), epsilon = EPSILON);
    }

    #[test]
    fn vector3_component_operations_work() {
        let v1 = Vector3::new(-1.0, 2.0, -3.0);
        let v2 = Vector3::new(4.0, -5.0, 6.0);

        assert_eq!(v1.component_abs(), Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(v1.component_mul(&v2), Vector3::new(-4.0, -10.0, -18.0));
        assert_eq!(v1.component_min(&v2), Vector3::new(-1.0, -5.0, -3.0));
        assert_eq!(v1.component_max(&v2), Vector3::new(4.0, 2.0, 6.0));
        assert_abs_diff_eq!(v1.min_component(), -3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v1.max_component(), 2.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3_swizzling_works() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(v.xy(), Vector2::new(1.0, 2.0));
        assert_eq!(v.yzx(), Vector3::new(2.0, 3.0, 1.0));
        assert_eq!(v.zxy(), Vector3::new(3.0, 1.0, 2.0));
        assert_eq!(v.yxx(), Vector3::new(2.0, 1.0, 1.0));
        assert_eq!(v.zzy(), Vector3::new(3.0, 3.0, 2.0));
    }

    #[test]
    fn extending_vector3_to_vector4_works() {
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let v4 = v3.extended(4.0);
        assert_eq!(v4, Vector4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn mapping_vector3_components_works() {
        let v = Vector3::new(1.0, -2.0, 3.0);
        let mapped = v.mapped(|x| x * 2.0);
        assert_eq!(mapped, Vector3::new(2.0, -4.0, 6.0));
    }

    #[test]
    fn vector3_compacting_and_alignment_works() {
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let compact = v3.compact();
        assert_eq!(compact, Vector3C::new(1.0, 2.0, 3.0));
        assert_eq!(compact.aligned(), v3);
    }

    #[test]
    fn vector3_arithmetic_operations_work() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);

        assert_eq!(&v1 + &v2, Vector3::new(5.0, 7.0, 9.0));
        assert_eq!(&v1 - &v2, Vector3::new(-3.0, -3.0, -3.0));
        assert_eq!(&v1 * 2.0, Vector3::new(2.0, 4.0, 6.0));
        assert_eq!(3.0 * &v1, Vector3::new(3.0, 6.0, 9.0));
        assert_eq!(&v1 / 2.0, Vector3::new(0.5, 1.0, 1.5));
        assert_eq!(-&v1, Vector3::new(-1.0, -2.0, -3.0));
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
        assert_eq!(v, Vector3::new(10.0, 20.0, 30.0));
    }

    #[test]
    #[should_panic]
    fn indexing_vector3_out_of_bounds_panics() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        let _ = v[3];
    }

    // === Vector3C Tests (compact) ===

    #[test]
    fn computing_vector3p_norm_works() {
        let v = Vector3C::new(3.0, 4.0, 0.0);
        assert_abs_diff_eq!(v.norm(), 5.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 25.0, epsilon = EPSILON);
    }

    #[test]
    fn normalizing_vector3p_gives_unit_vector() {
        let v = Vector3C::new(3.0, 4.0, 0.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized, Vector3C::new(0.6, 0.8, 0.0), epsilon = EPSILON);
    }

    #[test]
    fn vector3p_dot_product_works() {
        let v1 = Vector3C::new(1.0, 2.0, 3.0);
        let v2 = Vector3C::new(4.0, 5.0, 6.0);
        assert_abs_diff_eq!(v1.dot(&v2), 32.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3p_cross_product_works() {
        let v1 = Vector3C::new(1.0, 0.0, 0.0);
        let v2 = Vector3C::new(0.0, 1.0, 0.0);
        let cross = v1.cross(&v2);
        assert_abs_diff_eq!(cross, Vector3C::new(0.0, 0.0, 1.0), epsilon = EPSILON);
    }

    #[test]
    fn vector3p_cross_product_is_perpendicular() {
        let v1 = Vector3C::new(1.0, 2.0, 3.0);
        let v2 = Vector3C::new(4.0, 5.0, 6.0);
        let cross = v1.cross(&v2);

        assert_abs_diff_eq!(cross.dot(&v1), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.dot(&v2), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3p_cross_product_is_anticommutative() {
        let v1 = Vector3C::new(1.0, 2.0, 3.0);
        let v2 = Vector3C::new(4.0, 5.0, 6.0);
        let cross1 = v1.cross(&v2);
        let cross2 = v2.cross(&v1);

        assert_abs_diff_eq!(cross1, -&cross2, epsilon = EPSILON);
    }

    #[test]
    fn vector3p_cross_product_of_parallel_vectors_is_zero() {
        let v1 = Vector3C::new(1.0, 2.0, 3.0);
        let v2 = Vector3C::new(2.0, 4.0, 6.0);
        let cross = v1.cross(&v2);

        assert_abs_diff_eq!(cross, Vector3C::zeros(), epsilon = EPSILON);
    }

    #[test]
    fn vector3p_component_operations_work() {
        let v1 = Vector3C::new(-1.0, 2.0, -3.0);
        let v2 = Vector3C::new(4.0, -5.0, 6.0);

        assert_eq!(v1.component_abs(), Vector3C::new(1.0, 2.0, 3.0));
        assert_eq!(v1.component_mul(&v2), Vector3C::new(-4.0, -10.0, -18.0));
        assert_eq!(v1.component_min(&v2), Vector3C::new(-1.0, -5.0, -3.0));
        assert_eq!(v1.component_max(&v2), Vector3C::new(4.0, 2.0, 6.0));
        assert_abs_diff_eq!(v1.min_component(), -3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v1.max_component(), 2.0, epsilon = EPSILON);
    }

    #[test]
    fn mapping_vector3p_components_works() {
        let v = Vector3C::new(1.0, -2.0, 3.0);
        let mapped = v.mapped(|x| x * 2.0);
        assert_eq!(mapped, Vector3C::new(2.0, -4.0, 6.0));
    }

    #[test]
    fn extending_vector3p_to_vector4p_works() {
        let v3 = Vector3C::new(1.0, 2.0, 3.0);
        let v4 = v3.extended(4.0);
        assert_eq!(v4, Vector4C::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn vector3p_indexing_works() {
        let mut v = Vector3C::new(1.0, 2.0, 3.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);

        v[0] = 10.0;
        v[1] = 20.0;
        v[2] = 30.0;
        assert_eq!(v, Vector3C::new(10.0, 20.0, 30.0));
    }

    #[test]
    #[should_panic]
    fn indexing_vector3p_out_of_bounds_panics() {
        let v = Vector3C::new(1.0, 2.0, 3.0);
        let _ = v[3];
    }

    // === Vector4 Tests (SIMD-aligned) ===

    #[test]
    fn computing_vector4_norm_works() {
        let v = Vector4::new(1.0, 2.0, 2.0, 0.0);
        assert_abs_diff_eq!(v.norm(), 3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 9.0, epsilon = EPSILON);
    }

    #[test]
    fn normalizing_vector4_gives_unit_vector() {
        let v = Vector4::new(2.0, 0.0, 0.0, 0.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(
            normalized,
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            epsilon = EPSILON
        );
    }

    #[test]
    fn normalizing_zero_vector4_gives_nan() {
        let normalized = Vector4::zeros().normalized();
        assert!(normalized.x().is_nan());
    }

    #[test]
    fn vector4_dot_product_works() {
        let v1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        assert_abs_diff_eq!(v1.dot(&v2), 70.0, epsilon = EPSILON);
    }

    #[test]
    fn vector4_component_operations_work() {
        let v1 = Vector4::new(-1.0, 2.0, -3.0, 4.0);
        let v2 = Vector4::new(5.0, -6.0, 7.0, -8.0);

        assert_eq!(v1.component_abs(), Vector4::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(
            v1.component_mul(&v2),
            Vector4::new(-5.0, -12.0, -21.0, -32.0)
        );
        assert_eq!(v1.component_min(&v2), Vector4::new(-1.0, -6.0, -3.0, -8.0));
        assert_eq!(v1.component_max(&v2), Vector4::new(5.0, 2.0, 7.0, 4.0));
        assert_abs_diff_eq!(v1.min_component(), -3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v1.max_component(), 4.0, epsilon = EPSILON);
    }

    #[test]
    fn extracting_xyz_from_vector4_works() {
        let v4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let xyz = v4.xyz();
        assert_eq!(xyz, Vector3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn mapping_vector4_components_works() {
        let v = Vector4::new(1.0, -2.0, 3.0, -4.0);
        let mapped = v.mapped(|x| x * 2.0);
        assert_eq!(mapped, Vector4::new(2.0, -4.0, 6.0, -8.0));
    }

    #[test]
    fn vector4_compacting_and_alignment_works() {
        let v4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let compact = v4.compact();
        assert_eq!(compact, Vector4C::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(compact.aligned(), v4);
    }

    #[test]
    fn vector4_arithmetic_operations_work() {
        let v1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4::new(5.0, 6.0, 7.0, 8.0);

        assert_eq!(&v1 + &v2, Vector4::new(6.0, 8.0, 10.0, 12.0));
        assert_eq!(&v1 - &v2, Vector4::new(-4.0, -4.0, -4.0, -4.0));
        assert_eq!(&v1 * 2.0, Vector4::new(2.0, 4.0, 6.0, 8.0));
        assert_eq!(3.0 * &v1, Vector4::new(3.0, 6.0, 9.0, 12.0));
        assert_eq!(&v1 / 2.0, Vector4::new(0.5, 1.0, 1.5, 2.0));
        assert_eq!(-&v1, Vector4::new(-1.0, -2.0, -3.0, -4.0));
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
        assert_eq!(v, Vector4::new(10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    #[should_panic]
    fn indexing_vector4_out_of_bounds_panics() {
        let v = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let _ = v[4];
    }

    // === Vector4C Tests (compact) ===

    #[test]
    fn vector4p_indexing_works() {
        let mut v = Vector4C::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v[0], 1.0);
        assert_eq!(v[1], 2.0);
        assert_eq!(v[2], 3.0);
        assert_eq!(v[3], 4.0);

        v[0] = 10.0;
        v[1] = 20.0;
        v[2] = 30.0;
        v[3] = 40.0;
        assert_eq!(v, Vector4C::new(10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    #[should_panic]
    fn indexing_vector4p_out_of_bounds_panics() {
        let v = Vector4C::new(1.0, 2.0, 3.0, 4.0);
        let _ = v[4];
    }

    // === UnitVector3 Tests (SIMD-aligned) ===

    #[test]
    fn normalizing_vector3_creates_unitvector3() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let unit = UnitVector3::normalized_from(v);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.x(), 0.6, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.y(), 0.8, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn normalizing_zero_vector3_creates_nan_unitvector3() {
        let zero = Vector3::zeros();
        let unit = UnitVector3::normalized_from(zero);
        assert!(unit.norm().is_nan());
    }

    #[test]
    fn normalizing_vector3_if_above_threshold_works() {
        let v_large = Vector3::new(3.0, 4.0, 0.0);
        let unit_large = UnitVector3::normalized_from_if_above(v_large, 1.0);
        assert!(unit_large.is_some());
        assert_abs_diff_eq!(unit_large.unwrap().norm(), 1.0, epsilon = EPSILON);

        let v_small = Vector3::new(0.1, 0.1, 0.0);
        let unit_small = UnitVector3::normalized_from_if_above(v_small, 1.0);
        assert!(unit_small.is_none());
    }

    #[test]
    fn normalizing_vector3_and_getting_norm_works() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let (unit, norm) = UnitVector3::normalized_from_and_norm(v);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(norm, 5.0, epsilon = EPSILON);
    }

    #[test]
    fn normalizing_vector3_and_getting_norm_if_above_threshold_works() {
        let v_large = Vector3::new(3.0, 4.0, 0.0);
        let result_large = UnitVector3::normalized_from_and_norm_if_above(v_large, 1.0);
        assert!(result_large.is_some());
        let (unit, norm) = result_large.unwrap();
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(norm, 5.0, epsilon = EPSILON);

        let v_small = Vector3::new(0.1, 0.1, 0.0);
        let result_small = UnitVector3::normalized_from_and_norm_if_above(v_small, 1.0);
        assert!(result_small.is_none());
    }

    #[test]
    fn unitvector3_can_be_used_as_vector3_through_deref() {
        let unit = UnitVector3::unit_x();
        // These methods are available through Deref
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3_arithmetic_with_scalar_produces_vector3() {
        let unit = UnitVector3::unit_x();
        let scaled = &unit * 2.0;
        assert_eq!(scaled, Vector3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn unitvector3_compacting_and_alignment_works() {
        let unit = UnitVector3::unit_x();
        let compact = unit.compact();
        assert_eq!(compact, UnitVector3C::unit_x());
        assert_eq!(compact.aligned(), unit);
    }

    #[test]
    fn unitvector3_indexing_works() {
        let unit = UnitVector3::unit_y();
        assert_eq!(unit[0], 0.0);
        assert_eq!(unit[1], 1.0);
        assert_eq!(unit[2], 0.0);
    }

    // === UnitVector3C Tests (compact) ===

    #[test]
    fn normalizing_vector3p_creates_unitvector3p() {
        let v = Vector3C::new(3.0, 4.0, 0.0);
        let unit = UnitVector3C::normalized_from(v);

        let norm = (unit.x() * unit.x() + unit.y() * unit.y() + unit.z() * unit.z()).sqrt();
        assert_abs_diff_eq!(norm, 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.x(), 0.6, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.y(), 0.8, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3p_can_be_used_as_vector3p_through_deref() {
        let unit = UnitVector3C::unit_x();
        // These methods are available through Deref
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
    }

    #[test]
    fn unitvector3p_indexing_works() {
        let unit = UnitVector3C::unit_y();
        assert_eq!(unit[0], 0.0);
        assert_eq!(unit[1], 1.0);
        assert_eq!(unit[2], 0.0);
    }
}
