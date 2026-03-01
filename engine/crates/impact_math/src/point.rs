//! Points.

use crate::vector::{Vector2, Vector3, Vector3C};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;
use std::{
    fmt,
    ops::{Index, IndexMut, Mul, MulAssign},
};

/// A 2-dimensional point.
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Point2 {
    inner: glam::Vec2,
}

/// A 3-dimensional point.
///
/// The components are stored in a 128-bit SIMD register for efficient
/// computation. That leads to an extra 4 bytes in size and 16-byte alignment.
/// For cache-friendly storage, prefer the compact 4-byte aligned [`Point3C`].
#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Default, PartialEq, Zeroable, Pod)]
pub struct Point3 {
    inner: glam::Vec3A,
}

/// A 3-dimensional point. This is the "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Point3`].
#[repr(C)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "[f32; 3]", from = "[f32; 3]")
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Point3C {
    x: f32,
    y: f32,
    z: f32,
}

impl Point2 {
    /// Creates a new point with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self::wrap(glam::Vec2::new(x, y))
    }

    /// Creates a new point with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::new(value, value)
    }

    /// Creates a point at the origin.
    #[inline]
    pub const fn origin() -> Self {
        Self::wrap(glam::Vec2::ZERO)
    }

    /// Computes the center position between two points.
    #[inline]
    pub fn center_of(point_a: &Self, point_b: &Self) -> Self {
        Self::wrap(0.5 * (point_a.inner + point_b.inner))
    }

    /// This point as a [`Vector2`].
    #[inline]
    pub fn as_vector(&self) -> &Vector2 {
        bytemuck::cast_ref(self)
    }

    /// Returns a point where each component is the minimum of the corresponding
    /// components in this and another point.
    #[inline]
    pub fn min_with(&self, other: &Self) -> Self {
        Self::wrap(self.inner.min(other.inner))
    }

    /// Returns a point where each component is the maximum of the corresponding
    /// components in this and another point.
    #[inline]
    pub fn max_with(&self, other: &Self) -> Self {
        Self::wrap(self.inner.max(other.inner))
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

    /// Converts the point to 3D by appending the given z-component.
    #[inline]
    pub const fn extended(&self, z: f32) -> Point3C {
        Point3C::new(self.x(), self.y(), z)
    }

    /// Computes the distance between two points.
    #[inline]
    pub fn distance_between(point_a: &Self, point_b: &Self) -> f32 {
        Self::squared_distance_between(point_a, point_b).sqrt()
    }

    /// Computes the square of the distance between two points.
    #[inline]
    pub fn squared_distance_between(point_a: &Self, point_b: &Self) -> f32 {
        (point_b.inner - point_a.inner).length_squared()
    }

    #[inline]
    const fn wrap(inner: glam::Vec2) -> Self {
        Self { inner }
    }
}

impl From<Vector2> for Point2 {
    #[inline]
    fn from(vector: Vector2) -> Self {
        Self::new(vector.x(), vector.y())
    }
}

impl From<Point2> for Vector2 {
    #[inline]
    fn from(point: Point2) -> Self {
        Vector2::new(point.x(), point.y())
    }
}

impl From<[f32; 2]> for Point2 {
    #[inline]
    fn from([x, y]: [f32; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<Point2> for [f32; 2] {
    #[inline]
    fn from(point: Point2) -> Self {
        [point.x(), point.y()]
    }
}

impl_binop!(Add, add, Point2, Vector2, Point2, |a, b| {
    Point2::wrap(a.inner.add(b.unwrap()))
});

impl_binop!(Sub, sub, Point2, Vector2, Point2, |a, b| {
    Point2::wrap(a.inner.sub(b.unwrap()))
});

impl_binop!(Sub, sub, Point2, Point2, Vector2, |a, b| {
    Vector2::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Point2, f32, Point2, |a, b| {
    Point2::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Point2, Point2, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Point2, f32, Point2, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Point2, Vector2, |a, b| {
    a.inner.add_assign(b.unwrap());
});

impl_binop_assign!(SubAssign, sub_assign, Point2, Vector2, |a, b| {
    a.inner.sub_assign(b.unwrap());
});

impl_binop_assign!(MulAssign, mul_assign, Point2, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Point2, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl Index<usize> for Point2 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Point2 {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

impl fmt::Debug for Point2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point2")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .finish()
    }
}

impl_abs_diff_eq!(Point2, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Point2, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl Point3 {
    /// Creates a new point with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self::wrap(glam::Vec3A::new(x, y, z))
    }

    /// Creates a new point with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::new(value, value, value)
    }

    /// Creates a point at the origin.
    #[inline]
    pub const fn origin() -> Self {
        Self::wrap(glam::Vec3A::ZERO)
    }

    /// Computes the center position between two points.
    #[inline]
    pub fn center_of(point_a: &Self, point_b: &Self) -> Self {
        Self::wrap(0.5 * (point_a.inner + point_b.inner))
    }

    /// Returns a point where each component is the minimum of the corresponding
    /// components in this and another point.
    #[inline]
    pub fn min_with(&self, other: &Self) -> Self {
        Self::wrap(self.inner.min(other.inner))
    }

    /// Returns a point where each component is the maximum of the corresponding
    /// components in this and another point.
    #[inline]
    pub fn max_with(&self, other: &Self) -> Self {
        Self::wrap(self.inner.max(other.inner))
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

    /// The 2D point containing the x- and y-components of this point.
    #[inline]
    pub fn xy(&self) -> Point2 {
        Point2::new(self.x(), self.y())
    }

    /// Computes the distance between two points.
    #[inline]
    pub fn distance_between(point_a: &Self, point_b: &Self) -> f32 {
        Self::squared_distance_between(point_a, point_b).sqrt()
    }

    /// Computes the square of the distance between two points.
    #[inline]
    pub fn squared_distance_between(point_a: &Self, point_b: &Self) -> f32 {
        (point_b.inner - point_a.inner).length_squared()
    }

    /// This point as a [`Vector3`].
    #[inline]
    pub fn as_vector(&self) -> &Vector3 {
        bytemuck::cast_ref(self)
    }

    /// Converts the point to the 4-byte aligned cache-friendly [`Point3C`].
    #[inline]
    pub fn compact(&self) -> Point3C {
        Point3C::new(self.x(), self.y(), self.z())
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

impl From<Vector3> for Point3 {
    #[inline]
    fn from(vector: Vector3) -> Self {
        Self::new(vector.x(), vector.y(), vector.z())
    }
}

impl From<Point3> for Vector3 {
    #[inline]
    fn from(point: Point3) -> Self {
        Vector3::new(point.x(), point.y(), point.z())
    }
}

impl From<[f32; 3]> for Point3 {
    #[inline]
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Point3> for [f32; 3] {
    #[inline]
    fn from(point: Point3) -> Self {
        [point.x(), point.y(), point.z()]
    }
}

impl_binop!(Add, add, Point3, Vector3, Point3, |a, b| {
    Point3::wrap(a.inner.add(b.unwrap()))
});

impl_binop!(Sub, sub, Point3, Vector3, Point3, |a, b| {
    Point3::wrap(a.inner.sub(b.unwrap()))
});

impl_binop!(Sub, sub, Point3, Point3, Vector3, |a, b| {
    Vector3::wrap(a.inner.sub(b.inner))
});

impl_binop!(Mul, mul, Point3, f32, Point3, |a, b| {
    Point3::wrap(a.inner.mul(*b))
});

impl_binop!(Mul, mul, f32, Point3, Point3, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Point3, f32, Point3, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Point3, Vector3, |a, b| {
    a.inner.add_assign(b.unwrap());
});

impl_binop_assign!(SubAssign, sub_assign, Point3, Vector3, |a, b| {
    a.inner.sub_assign(b.unwrap());
});

impl_binop_assign!(MulAssign, mul_assign, Point3, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Point3, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl Index<usize> for Point3 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Point3 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

impl fmt::Debug for Point3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point3")
            .field("x", &self.inner.x)
            .field("y", &self.inner.y)
            .field("z", &self.inner.z)
            .finish()
    }
}

impl_abs_diff_eq!(Point3, |a, b, epsilon| {
    a.inner.abs_diff_eq(b.inner, epsilon)
});

impl_relative_eq!(Point3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl Point3C {
    /// Creates a new point with the given components.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Creates a new point with the same value for all components.
    #[inline]
    pub const fn same(value: f32) -> Self {
        Self::new(value, value, value)
    }

    /// Creates a point at the origin.
    #[inline]
    pub const fn origin() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Computes the center position between two points.
    #[inline]
    pub fn center_of(point_a: &Self, point_b: &Self) -> Self {
        Self {
            x: 0.5 * (point_a.x + point_b.x),
            y: 0.5 * (point_a.y + point_b.y),
            z: 0.5 * (point_a.z + point_b.z),
        }
    }

    /// Returns a point where each component is the minimum of the corresponding
    /// components in this and another point.
    #[inline]
    pub fn min_with(&self, other: &Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    /// Returns a point where each component is the maximum of the corresponding
    /// components in this and another point.
    #[inline]
    pub fn max_with(&self, other: &Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
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

    /// The 2D point containing the x- and y-components of this point.
    #[inline]
    pub fn xy(&self) -> Point2 {
        Point2::new(self.x(), self.y())
    }

    /// This point as a [`Vector3C`].
    #[inline]
    pub fn as_vector(&self) -> &Vector3C {
        bytemuck::cast_ref(self)
    }

    /// Converts the point to the 16-byte aligned SIMD-friendly [`Point3`].
    #[inline]
    pub fn aligned(&self) -> Point3 {
        Point3::new(self.x(), self.y(), self.z())
    }
}

impl From<Vector3C> for Point3C {
    #[inline]
    fn from(vector: Vector3C) -> Self {
        Self::new(vector.x(), vector.y(), vector.z())
    }
}

impl From<Point3C> for Vector3C {
    #[inline]
    fn from(point: Point3C) -> Self {
        Vector3C::new(point.x(), point.y(), point.z())
    }
}

impl From<[f32; 3]> for Point3C {
    #[inline]
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Point3C> for [f32; 3] {
    #[inline]
    fn from(point: Point3C) -> Self {
        [point.x(), point.y(), point.z()]
    }
}

impl_binop!(Add, add, Point3C, Vector3C, Point3C, |a, b| {
    Point3C::new(a.x + b.x(), a.y + b.y(), a.z + b.z())
});

impl_binop!(Sub, sub, Point3C, Vector3C, Point3C, |a, b| {
    Point3C::new(a.x - b.x(), a.y - b.y(), a.z - b.z())
});

impl_binop!(Sub, sub, Point3C, Point3C, Vector3C, |a, b| {
    Vector3C::new(a.x - b.x, a.y - b.y, a.z - b.z)
});

impl_binop!(Mul, mul, Point3C, f32, Point3C, |a, b| {
    Point3C::new(a.x * b, a.y * b, a.z * b)
});

impl_binop!(Mul, mul, f32, Point3C, Point3C, |a, b| { b.mul(*a) });

impl_binop!(Div, div, Point3C, f32, Point3C, |a, b| { a.mul(b.recip()) });

impl_binop_assign!(AddAssign, add_assign, Point3C, Vector3C, |a, b| {
    a.x += b.x();
    a.y += b.y();
    a.z += b.z();
});

impl_binop_assign!(SubAssign, sub_assign, Point3C, Vector3C, |a, b| {
    a.x -= b.x();
    a.y -= b.y();
    a.z -= b.z();
});

impl_binop_assign!(MulAssign, mul_assign, Point3C, f32, |a, b| {
    a.x *= b;
    a.y *= b;
    a.z *= b;
});

impl_binop_assign!(DivAssign, div_assign, Point3C, f32, |a, b| {
    a.mul_assign(b.recip());
});

impl Index<usize> for Point3C {
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

impl IndexMut<usize> for Point3C {
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

impl_abs_diff_eq!(Point3C, |a, b, epsilon| {
    a.x.abs_diff_eq(&b.x, epsilon)
        && a.y.abs_diff_eq(&b.y, epsilon)
        && a.z.abs_diff_eq(&b.z, epsilon)
});

impl_relative_eq!(Point3C, |a, b, epsilon, max_relative| {
    a.x.relative_eq(&b.x, epsilon, max_relative)
        && a.y.relative_eq(&b.y, epsilon, max_relative)
        && a.z.relative_eq(&b.z, epsilon, max_relative)
});

impl_roc_for_library_provided_primitives! {
//  Type        Pkg    Parents  Module  Roc name  Postfix  Precision
    Point2   => core,  None,    Point2, Point2,   None,    PrecisionIrrelevant,
    Point3C  => core,  None,    Point3, Point3,   None,    PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use approx::assert_abs_diff_eq;

    const EPSILON: f32 = 1e-6;

    // === Point2 Tests ===

    #[test]
    fn calculating_point2_center_works() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 6.0);
        let center = Point2::center_of(&p1, &p2);
        assert_eq!(center, Point2::new(2.0, 3.0));
    }

    #[test]
    fn calculating_point2_center_with_negative_coordinates_works() {
        let p1 = Point2::new(-2.0, -4.0);
        let p2 = Point2::new(2.0, 4.0);
        let center = Point2::center_of(&p1, &p2);
        assert_eq!(center, Point2::origin());
    }

    #[test]
    fn point2_center_is_equidistant_from_both_points() {
        let p1 = Point2::new(1.0, 2.0);
        let p2 = Point2::new(5.0, 8.0);
        let center = Point2::center_of(&p1, &p2);

        let d1 = Point2::distance_between(&p1, &center);
        let d2 = Point2::distance_between(&p2, &center);
        assert_abs_diff_eq!(d1, d2, epsilon = EPSILON);
    }

    #[test]
    fn point2_min_max_with_work() {
        let p1 = Point2::new(1.0, 4.0);
        let p2 = Point2::new(3.0, 2.0);

        assert_eq!(p1.min_with(&p2), Point2::new(1.0, 2.0));
        assert_eq!(p1.max_with(&p2), Point2::new(3.0, 4.0));
    }

    #[test]
    fn point2_min_max_with_same_point_gives_same_point() {
        let p = Point2::new(1.0, 2.0);
        assert_eq!(p.min_with(&p), p);
        assert_eq!(p.max_with(&p), p);
    }

    #[test]
    fn calculating_point2_distance_works() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(3.0, 4.0);

        assert_abs_diff_eq!(Point2::distance_between(&p1, &p2), 5.0, epsilon = EPSILON);
        assert_abs_diff_eq!(
            Point2::squared_distance_between(&p1, &p2),
            25.0,
            epsilon = EPSILON
        );
    }

    #[test]
    fn point2_distance_is_symmetric() {
        let p1 = Point2::new(1.0, 2.0);
        let p2 = Point2::new(4.0, 6.0);

        let d1 = Point2::distance_between(&p1, &p2);
        let d2 = Point2::distance_between(&p2, &p1);
        assert_abs_diff_eq!(d1, d2, epsilon = EPSILON);
    }

    #[test]
    fn converting_point2_to_vector_and_back_preserves_data() {
        let p = Point2::new(3.0, 4.0);
        let v = p.as_vector();
        let p_back = Point2::from(*v);
        assert_eq!(p_back, p);
    }

    #[test]
    fn extending_point2_to_point3p_works() {
        let p2 = Point2::new(1.0, 2.0);
        let p3 = p2.extended(3.0);
        assert_eq!(p3, Point3C::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn point2_arithmetic_with_vector_works() {
        let p = Point2::new(1.0, 2.0);
        let v = Vector2::new(3.0, 4.0);

        assert_eq!(&p + &v, Point2::new(4.0, 6.0));
        assert_eq!(&p - &v, Point2::new(-2.0, -2.0));
    }

    #[test]
    fn point2_arithmetic_with_zero_vector_gives_same_point() {
        let p = Point2::new(3.0, 4.0);
        let zero = Vector2::zeros();
        assert_eq!(&p + &zero, p);
        assert_eq!(&p - &zero, p);
    }

    #[test]
    fn point2_arithmetic_with_scalar_works() {
        let p = Point2::new(2.0, 3.0);

        assert_eq!(&p * 2.0, Point2::new(4.0, 6.0));
        assert_eq!(3.0 * &p, Point2::new(6.0, 9.0));
        assert_eq!(&p / 2.0, Point2::new(1.0, 1.5));
    }

    #[test]
    fn point2_scalar_multiplication_by_one_gives_same_point() {
        let p = Point2::new(3.0, 4.0);
        assert_eq!(&p * 1.0, p);
    }

    #[test]
    fn point2_subtraction_gives_vector() {
        let p1 = Point2::new(5.0, 7.0);
        let p2 = Point2::new(2.0, 3.0);
        let diff = &p1 - &p2;
        assert_eq!(diff, Vector2::new(3.0, 4.0));
    }

    #[test]
    fn point2_indexing_works() {
        let mut p = Point2::new(1.0, 2.0);
        assert_eq!(p[0], 1.0);
        assert_eq!(p[1], 2.0);

        p[0] = 10.0;
        p[1] = 20.0;
        assert_eq!(p, Point2::new(10.0, 20.0));
    }

    #[test]
    #[should_panic]
    fn indexing_point2_out_of_bounds_panics() {
        let p = Point2::new(1.0, 2.0);
        let _ = p[2];
    }

    // === Point3 Tests (SIMD-aligned) ===

    #[test]
    fn calculating_point3_center_works() {
        let p1 = Point3::new(0.0, 0.0, 0.0);
        let p2 = Point3::new(4.0, 6.0, 8.0);
        let center = Point3::center_of(&p1, &p2);
        assert_eq!(center, Point3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn calculating_point3_center_with_negative_coordinates_works() {
        let p1 = Point3::new(-2.0, -4.0, -6.0);
        let p2 = Point3::new(2.0, 4.0, 6.0);
        let center = Point3::center_of(&p1, &p2);
        assert_eq!(center, Point3::origin());
    }

    #[test]
    fn point3_center_is_equidistant_from_both_points() {
        let p1 = Point3::new(1.0, 2.0, 3.0);
        let p2 = Point3::new(5.0, 8.0, 11.0);
        let center = Point3::center_of(&p1, &p2);

        let d1 = Point3::distance_between(&p1, &center);
        let d2 = Point3::distance_between(&p2, &center);
        assert_abs_diff_eq!(d1, d2, epsilon = EPSILON);
    }

    #[test]
    fn point3_min_max_with_work() {
        let p1 = Point3::new(1.0, 4.0, 2.0);
        let p2 = Point3::new(3.0, 2.0, 5.0);

        assert_eq!(p1.min_with(&p2), Point3::new(1.0, 2.0, 2.0));
        assert_eq!(p1.max_with(&p2), Point3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn point3_min_max_with_negative_values_work() {
        let p1 = Point3::new(-1.0, -4.0, -2.0);
        let p2 = Point3::new(-3.0, -2.0, -5.0);

        assert_eq!(p1.min_with(&p2), Point3::new(-3.0, -4.0, -5.0));
        assert_eq!(p1.max_with(&p2), Point3::new(-1.0, -2.0, -2.0));
    }

    #[test]
    fn calculating_point3_distance_works() {
        let p1 = Point3::new(0.0, 0.0, 0.0);
        let p2 = Point3::new(1.0, 2.0, 2.0);

        assert_abs_diff_eq!(Point3::distance_between(&p1, &p2), 3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(
            Point3::squared_distance_between(&p1, &p2),
            9.0,
            epsilon = EPSILON
        );
    }

    #[test]
    fn point3_distance_is_symmetric() {
        let p1 = Point3::new(1.0, 2.0, 3.0);
        let p2 = Point3::new(4.0, 6.0, 8.0);

        let d1 = Point3::distance_between(&p1, &p2);
        let d2 = Point3::distance_between(&p2, &p1);
        assert_abs_diff_eq!(d1, d2, epsilon = EPSILON);
    }

    #[test]
    fn distance_between_point3_and_origin_works() {
        let origin = Point3::origin();
        let p = Point3::new(3.0, 4.0, 0.0);
        assert_abs_diff_eq!(
            Point3::distance_between(&origin, &p),
            5.0,
            epsilon = EPSILON
        );
    }

    #[test]
    fn extracting_xy_from_point3_works() {
        let p3 = Point3::new(1.0, 2.0, 3.0);
        let xy = p3.xy();
        assert_eq!(xy, Point2::new(1.0, 2.0));
    }

    #[test]
    fn converting_point3_to_vector_and_back_preserves_data() {
        let p = Point3::new(3.0, 4.0, 5.0);
        let v = p.as_vector();
        let p_back = Point3::from(*v);
        assert_eq!(p_back, p);
    }

    #[test]
    fn point3_compacting_and_alignment_works() {
        let p3 = Point3::new(1.0, 2.0, 3.0);
        let compact = p3.compact();
        assert_eq!(compact, Point3C::new(1.0, 2.0, 3.0));
        assert_eq!(compact.aligned(), p3);
    }

    #[test]
    fn point3_arithmetic_with_vector_works() {
        let p = Point3::new(1.0, 2.0, 3.0);
        let v = Vector3::new(4.0, 5.0, 6.0);

        assert_eq!(&p + &v, Point3::new(5.0, 7.0, 9.0));
        assert_eq!(&p - &v, Point3::new(-3.0, -3.0, -3.0));
    }

    #[test]
    fn point3_arithmetic_with_zero_vector_gives_same_point() {
        let p = Point3::new(3.0, 4.0, 5.0);
        let zero = Vector3::zeros();
        assert_eq!(&p + &zero, p);
        assert_eq!(&p - &zero, p);
    }

    #[test]
    fn point3_arithmetic_with_scalar_works() {
        let p = Point3::new(2.0, 3.0, 4.0);

        assert_eq!(&p * 2.0, Point3::new(4.0, 6.0, 8.0));
        assert_eq!(3.0 * &p, Point3::new(6.0, 9.0, 12.0));
        assert_eq!(&p / 2.0, Point3::new(1.0, 1.5, 2.0));
    }

    #[test]
    fn point3_scalar_multiplication_by_one_gives_same_point() {
        let p = Point3::new(3.0, 4.0, 5.0);
        assert_eq!(&p * 1.0, p);
    }

    #[test]
    fn point3_scalar_multiplication_by_negative_works() {
        let p = Point3::new(2.0, -3.0, 4.0);
        assert_eq!(&p * -1.0, Point3::new(-2.0, 3.0, -4.0));
    }

    #[test]
    fn point3_subtraction_gives_vector() {
        let p1 = Point3::new(5.0, 7.0, 9.0);
        let p2 = Point3::new(2.0, 3.0, 4.0);
        let diff = &p1 - &p2;
        assert_eq!(diff, Vector3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn point3_indexing_works() {
        let mut p = Point3::new(1.0, 2.0, 3.0);
        assert_eq!(p[0], 1.0);
        assert_eq!(p[1], 2.0);
        assert_eq!(p[2], 3.0);

        p[0] = 10.0;
        p[1] = 20.0;
        p[2] = 30.0;
        assert_eq!(p, Point3::new(10.0, 20.0, 30.0));
    }

    #[test]
    #[should_panic]
    fn indexing_point3_out_of_bounds_panics() {
        let p = Point3::new(1.0, 2.0, 3.0);
        let _ = p[3];
    }

    // === Point3C Tests (compact) ===

    #[test]
    fn extracting_xy_from_point3p_works() {
        let p3 = Point3C::new(1.0, 2.0, 3.0);
        let xy = p3.xy();
        assert_eq!(xy, Point2::new(1.0, 2.0));
    }

    #[test]
    fn converting_point3p_to_vector_and_back_preserves_data() {
        let p = Point3C::new(3.0, 4.0, 5.0);
        let v = p.as_vector();
        let p_back = Point3C::from(*v);
        assert_eq!(p_back, p);
    }

    #[test]
    fn point3p_indexing_works() {
        let mut p = Point3C::new(1.0, 2.0, 3.0);
        assert_eq!(p[0], 1.0);
        assert_eq!(p[1], 2.0);
        assert_eq!(p[2], 3.0);

        p[0] = 10.0;
        p[1] = 20.0;
        p[2] = 30.0;
        assert_eq!(p, Point3C::new(10.0, 20.0, 30.0));
    }

    #[test]
    #[should_panic]
    fn indexing_point3p_out_of_bounds_panics() {
        let p = Point3C::new(1.0, 2.0, 3.0);
        let _ = p[3];
    }

    // === General Point Tests ===

    #[test]
    fn point_arithmetic_maintains_precision() {
        let p = Point3::new(0.1, 0.2, 0.3);
        let doubled = &p * 2.0;
        let halved = &doubled / 2.0;

        assert_abs_diff_eq!(halved.x(), p.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.y(), p.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.z(), p.z(), epsilon = EPSILON);
    }
}
