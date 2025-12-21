//! Points.

use std::ops::{Index, IndexMut};

use crate::vector::{Vector2, Vector3};
use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Point2 {
    inner: nalgebra::Point2<f32>,
}

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Point3 {
    inner: nalgebra::Point3<f32>,
}

impl Point2 {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            inner: nalgebra::Point2::new(x, y),
        }
    }

    #[inline]
    pub fn origin() -> Self {
        Self {
            inner: nalgebra::Point2::origin(),
        }
    }

    #[inline]
    pub fn center_of(point_a: &Self, point_b: &Self) -> Self {
        Self {
            inner: nalgebra::center(&point_a.inner, &point_b.inner),
        }
    }

    #[inline]
    pub fn as_vector(&self) -> &Vector2 {
        bytemuck::from_bytes(bytemuck::bytes_of(&self.inner))
    }

    #[inline]
    pub fn min_with(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.inf(&other.inner))
    }

    #[inline]
    pub fn max_with(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.sup(&other.inner))
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.inner.y
    }

    #[inline]
    pub fn x_mut(&mut self) -> &mut f32 {
        &mut self.inner.x
    }

    #[inline]
    pub fn y_mut(&mut self) -> &mut f32 {
        &mut self.inner.y
    }

    #[inline]
    pub fn distance_between(point_a: &Self, point_b: &Self) -> f32 {
        nalgebra::distance(&point_a.inner, &point_b.inner)
    }

    #[inline]
    pub fn squared_distance_between(point_a: &Self, point_b: &Self) -> f32 {
        nalgebra::distance_squared(&point_a.inner, &point_b.inner)
    }

    #[inline]
    pub fn _wrap(inner: nalgebra::Point2<f32>) -> Self {
        Self { inner }
    }
}

impl From<Vector2> for Point2 {
    #[inline]
    fn from(vector: Vector2) -> Self {
        Self {
            inner: (*vector._inner()).into(),
        }
    }
}

impl From<Point2> for Vector2 {
    #[inline]
    fn from(point: Point2) -> Self {
        Vector2::_wrap(point.inner.coords)
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
    fn from(vector: Point2) -> Self {
        [vector.x(), vector.y()]
    }
}

impl_binop!(Add, add, Point2, Vector2, Point2, |a, b| {
    Point2 {
        inner: a.inner + b._inner(),
    }
});

impl_binop!(Sub, sub, Point2, Vector2, Point2, |a, b| {
    Point2 {
        inner: a.inner - b._inner(),
    }
});

impl_binop!(Mul, mul, Point2, f32, Point2, |a, b| {
    Point2 {
        inner: a.inner * *b,
    }
});

impl_binop!(Mul, mul, f32, Point2, Point2, |a, b| {
    Point2 {
        inner: *a * b.inner,
    }
});

impl_binop!(Div, div, Point2, f32, Point2, |a, b| {
    #[allow(clippy::suspicious_arithmetic_impl)]
    Point2 {
        inner: a.inner * b.recip(),
    }
});

impl_binop!(Sub, sub, Point2, Point2, Vector2, |a, b| {
    Vector2::_wrap(a.inner - b.inner)
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

impl_abs_diff_eq!(Point2, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Point2, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl Point3 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            inner: nalgebra::Point3::new(x, y, z),
        }
    }

    #[inline]
    pub fn origin() -> Self {
        Self {
            inner: nalgebra::Point3::origin(),
        }
    }

    #[inline]
    pub fn center_of(point_a: &Self, point_b: &Self) -> Self {
        Self {
            inner: nalgebra::center(&point_a.inner, &point_b.inner),
        }
    }

    #[inline]
    pub fn as_vector(&self) -> &Vector3 {
        bytemuck::from_bytes(bytemuck::bytes_of(&self.inner))
    }

    #[inline]
    pub fn min_with(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.inf(&other.inner))
    }

    #[inline]
    pub fn max_with(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.sup(&other.inner))
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.inner.x
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.inner.y
    }

    #[inline]
    pub fn z(&self) -> f32 {
        self.inner.z
    }

    #[inline]
    pub fn x_mut(&mut self) -> &mut f32 {
        &mut self.inner.x
    }

    #[inline]
    pub fn y_mut(&mut self) -> &mut f32 {
        &mut self.inner.y
    }

    #[inline]
    pub fn z_mut(&mut self) -> &mut f32 {
        &mut self.inner.z
    }

    #[inline]
    pub fn xy(&self) -> Point2 {
        Point2::_wrap(self.inner.xy())
    }

    #[inline]
    pub fn distance_between(point_a: &Self, point_b: &Self) -> f32 {
        nalgebra::distance(&point_a.inner, &point_b.inner)
    }

    #[inline]
    pub fn squared_distance_between(point_a: &Self, point_b: &Self) -> f32 {
        nalgebra::distance_squared(&point_a.inner, &point_b.inner)
    }

    #[inline]
    pub fn _wrap(inner: nalgebra::Point3<f32>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn _inner(&self) -> &nalgebra::Point3<f32> {
        &self.inner
    }
}

impl From<Vector3> for Point3 {
    #[inline]
    fn from(vector: Vector3) -> Self {
        Self {
            inner: (*vector._inner()).into(),
        }
    }
}

impl From<Point3> for Vector3 {
    #[inline]
    fn from(point: Point3) -> Self {
        Vector3::_wrap(point.inner.coords)
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
    fn from(vector: Point3) -> Self {
        [vector.x(), vector.y(), vector.z()]
    }
}

impl_binop!(Add, add, Point3, Vector3, Point3, |a, b| {
    Point3 {
        inner: a.inner + b._inner(),
    }
});

impl std::ops::AddAssign<Vector3> for Point3 {
    fn add_assign(&mut self, rhs: Vector3) {
        self.inner.add_assign(rhs._inner());
    }
}
impl std::ops::AddAssign<&Vector3> for Point3 {
    fn add_assign(&mut self, rhs: &Vector3) {
        self.inner.add_assign(rhs._inner());
    }
}

impl_binop!(Sub, sub, Point3, Vector3, Point3, |a, b| {
    Point3 {
        inner: a.inner - b._inner(),
    }
});

impl_binop!(Sub, sub, Point3, Point3, Vector3, |a, b| {
    Vector3::_wrap(a.inner - b.inner)
});

impl_binop!(Mul, mul, Point3, f32, Point3, |a, b| {
    Point3 {
        inner: a.inner * *b,
    }
});

impl_binop!(Mul, mul, f32, Point3, Point3, |a, b| {
    Point3 {
        inner: *a * b.inner,
    }
});

impl_binop!(Div, div, Point3, f32, Point3, |a, b| {
    #[allow(clippy::suspicious_arithmetic_impl)]
    Point3 {
        inner: a.inner * b.recip(),
    }
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

impl_abs_diff_eq!(Point3, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Point3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl_roc_for_library_provided_primitives! {
//  Type       Pkg    Parents  Module  Roc name  Postfix  Precision
    Point2  => core,  None,    Point2, Point2,   None,    PrecisionIrrelevant,
    Point3  => core,  None,    Point3, Point3,   None,    PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::op_ref)]

    use super::*;
    use crate::vector::{Vector2, Vector3};
    use approx::assert_abs_diff_eq;

    // Test constants
    const EPSILON: f32 = 1e-6;

    // Point2 tests
    #[test]
    fn point2_new_works() {
        let p = Point2::new(1.0, 2.0);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);
    }

    #[test]
    fn point2_origin_gives_zero_point() {
        let origin = Point2::origin();
        assert_eq!(origin.x(), 0.0);
        assert_eq!(origin.y(), 0.0);
    }

    #[test]
    fn point2_center_of_calculates_midpoint() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(4.0, 6.0);
        let center = Point2::center_of(&p1, &p2);
        assert_eq!(center.x(), 2.0);
        assert_eq!(center.y(), 3.0);
    }

    #[test]
    fn point2_component_accessors_work() {
        let mut p = Point2::new(1.0, 2.0);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);

        *p.x_mut() = 10.0;
        *p.y_mut() = 20.0;
        assert_eq!(p.x(), 10.0);
        assert_eq!(p.y(), 20.0);
    }

    #[test]
    fn point2_as_vector_works() {
        let p = Point2::new(3.0, 4.0);
        let v = p.as_vector();
        assert_eq!(v.x(), 3.0);
        assert_eq!(v.y(), 4.0);
    }

    #[test]
    fn point2_min_max_with_work() {
        let p1 = Point2::new(1.0, 4.0);
        let p2 = Point2::new(3.0, 2.0);

        let min_p = p1.min_with(&p2);
        assert_eq!(min_p.x(), 1.0);
        assert_eq!(min_p.y(), 2.0);

        let max_p = p1.max_with(&p2);
        assert_eq!(max_p.x(), 3.0);
        assert_eq!(max_p.y(), 4.0);
    }

    #[test]
    fn point2_distance_calculations_work() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(3.0, 4.0);

        let distance = Point2::distance_between(&p1, &p2);
        assert_abs_diff_eq!(distance, 5.0, epsilon = EPSILON);

        let squared_distance = Point2::squared_distance_between(&p1, &p2);
        assert_abs_diff_eq!(squared_distance, 25.0, epsilon = EPSILON);
    }

    #[test]
    fn point2_vector_conversions_work() {
        let v = Vector2::new(2.0, 3.0);
        let p = Point2::from(v);
        assert_eq!(p.x(), 2.0);
        assert_eq!(p.y(), 3.0);

        let v_back = Vector2::from(p);
        assert_eq!(v_back.x(), 2.0);
        assert_eq!(v_back.y(), 3.0);
    }

    #[test]
    fn point2_array_conversions_work() {
        let arr: [f32; 2] = [1.0, 2.0];
        let p = Point2::from(arr);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);

        let arr_back: [f32; 2] = p.into();
        assert_eq!(arr_back, [1.0, 2.0]);
    }

    #[test]
    fn point2_arithmetic_with_vector_works() {
        let p = Point2::new(1.0, 2.0);
        let v = Vector2::new(3.0, 4.0);

        let add_result = &p + &v;
        assert_eq!(add_result.x(), 4.0);
        assert_eq!(add_result.y(), 6.0);

        let sub_result = &p - &v;
        assert_eq!(sub_result.x(), -2.0);
        assert_eq!(sub_result.y(), -2.0);
    }

    #[test]
    fn point2_arithmetic_with_scalar_works() {
        let p = Point2::new(2.0, 3.0);

        let mul_result = &p * 2.0;
        assert_eq!(mul_result.x(), 4.0);
        assert_eq!(mul_result.y(), 6.0);

        let scalar_mul = 3.0 * &p;
        assert_eq!(scalar_mul.x(), 6.0);
        assert_eq!(scalar_mul.y(), 9.0);

        let div_result = &p / 2.0;
        assert_eq!(div_result.x(), 1.0);
        assert_eq!(div_result.y(), 1.5);
    }

    #[test]
    fn point2_subtraction_gives_vector() {
        let p1 = Point2::new(5.0, 7.0);
        let p2 = Point2::new(2.0, 3.0);

        let diff = &p1 - &p2;
        assert_eq!(diff.x(), 3.0);
        assert_eq!(diff.y(), 4.0);
    }

    #[test]
    fn point2_indexing_works() {
        let mut p = Point2::new(1.0, 2.0);
        assert_eq!(p[0], 1.0);
        assert_eq!(p[1], 2.0);

        p[0] = 10.0;
        p[1] = 20.0;
        assert_eq!(p[0], 10.0);
        assert_eq!(p[1], 20.0);
    }

    #[test]
    #[should_panic]
    fn point2_indexing_panics_on_out_of_bounds() {
        let p = Point2::new(1.0, 2.0);
        let _ = p[2]; // Should panic
    }

    // Point3 tests
    #[test]
    fn point3_new_works() {
        let p = Point3::new(1.0, 2.0, 3.0);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);
        assert_eq!(p.z(), 3.0);
    }

    #[test]
    fn point3_origin_gives_zero_point() {
        let origin = Point3::origin();
        assert_eq!(origin.x(), 0.0);
        assert_eq!(origin.y(), 0.0);
        assert_eq!(origin.z(), 0.0);
    }

    #[test]
    fn point3_center_of_calculates_midpoint() {
        let p1 = Point3::new(0.0, 0.0, 0.0);
        let p2 = Point3::new(4.0, 6.0, 8.0);
        let center = Point3::center_of(&p1, &p2);
        assert_eq!(center.x(), 2.0);
        assert_eq!(center.y(), 3.0);
        assert_eq!(center.z(), 4.0);
    }

    #[test]
    fn point3_component_accessors_work() {
        let mut p = Point3::new(1.0, 2.0, 3.0);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);
        assert_eq!(p.z(), 3.0);

        *p.x_mut() = 10.0;
        *p.y_mut() = 20.0;
        *p.z_mut() = 30.0;
        assert_eq!(p.x(), 10.0);
        assert_eq!(p.y(), 20.0);
        assert_eq!(p.z(), 30.0);
    }

    #[test]
    fn point3_as_vector_works() {
        let p = Point3::new(3.0, 4.0, 5.0);
        let v = p.as_vector();
        assert_eq!(v.x(), 3.0);
        assert_eq!(v.y(), 4.0);
        assert_eq!(v.z(), 5.0);
    }

    #[test]
    fn point3_min_max_with_work() {
        let p1 = Point3::new(1.0, 4.0, 2.0);
        let p2 = Point3::new(3.0, 2.0, 5.0);

        let min_p = p1.min_with(&p2);
        assert_eq!(min_p.x(), 1.0);
        assert_eq!(min_p.y(), 2.0);
        assert_eq!(min_p.z(), 2.0);

        let max_p = p1.max_with(&p2);
        assert_eq!(max_p.x(), 3.0);
        assert_eq!(max_p.y(), 4.0);
        assert_eq!(max_p.z(), 5.0);
    }

    #[test]
    fn point3_xy_extraction_works() {
        let p3 = Point3::new(1.0, 2.0, 3.0);
        let xy = p3.xy();
        assert_eq!(xy.x(), 1.0);
        assert_eq!(xy.y(), 2.0);
    }

    #[test]
    fn point3_distance_calculations_work() {
        let p1 = Point3::new(0.0, 0.0, 0.0);
        let p2 = Point3::new(1.0, 2.0, 2.0);

        let distance = Point3::distance_between(&p1, &p2);
        assert_abs_diff_eq!(distance, 3.0, epsilon = EPSILON);

        let squared_distance = Point3::squared_distance_between(&p1, &p2);
        assert_abs_diff_eq!(squared_distance, 9.0, epsilon = EPSILON);
    }

    #[test]
    fn point3_vector_conversions_work() {
        let v = Vector3::new(2.0, 3.0, 4.0);
        let p = Point3::from(v);
        assert_eq!(p.x(), 2.0);
        assert_eq!(p.y(), 3.0);
        assert_eq!(p.z(), 4.0);

        let v_back = Vector3::from(p);
        assert_eq!(v_back.x(), 2.0);
        assert_eq!(v_back.y(), 3.0);
        assert_eq!(v_back.z(), 4.0);
    }

    #[test]
    fn point3_array_conversions_work() {
        let arr: [f32; 3] = [1.0, 2.0, 3.0];
        let p = Point3::from(arr);
        assert_eq!(p.x(), 1.0);
        assert_eq!(p.y(), 2.0);
        assert_eq!(p.z(), 3.0);

        let arr_back: [f32; 3] = p.into();
        assert_eq!(arr_back, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn point3_arithmetic_with_vector_works() {
        let p = Point3::new(1.0, 2.0, 3.0);
        let v = Vector3::new(4.0, 5.0, 6.0);

        let add_result = &p + &v;
        assert_eq!(add_result.x(), 5.0);
        assert_eq!(add_result.y(), 7.0);
        assert_eq!(add_result.z(), 9.0);

        let sub_result = &p - &v;
        assert_eq!(sub_result.x(), -3.0);
        assert_eq!(sub_result.y(), -3.0);
        assert_eq!(sub_result.z(), -3.0);
    }

    #[test]
    fn point3_add_assign_operations_work() {
        let mut p = Point3::new(1.0, 2.0, 3.0);
        let v = Vector3::new(4.0, 5.0, 6.0);

        p += &v;
        assert_eq!(p.x(), 5.0);
        assert_eq!(p.y(), 7.0);
        assert_eq!(p.z(), 9.0);

        p += v;
        assert_eq!(p.x(), 9.0);
        assert_eq!(p.y(), 12.0);
        assert_eq!(p.z(), 15.0);
    }

    #[test]
    fn point3_arithmetic_with_scalar_works() {
        let p = Point3::new(2.0, 3.0, 4.0);

        let mul_result = &p * 2.0;
        assert_eq!(mul_result.x(), 4.0);
        assert_eq!(mul_result.y(), 6.0);
        assert_eq!(mul_result.z(), 8.0);

        let scalar_mul = 3.0 * &p;
        assert_eq!(scalar_mul.x(), 6.0);
        assert_eq!(scalar_mul.y(), 9.0);
        assert_eq!(scalar_mul.z(), 12.0);

        let div_result = &p / 2.0;
        assert_eq!(div_result.x(), 1.0);
        assert_eq!(div_result.y(), 1.5);
        assert_eq!(div_result.z(), 2.0);
    }

    #[test]
    fn point3_subtraction_gives_vector() {
        let p1 = Point3::new(5.0, 7.0, 9.0);
        let p2 = Point3::new(2.0, 3.0, 4.0);

        let diff = &p1 - &p2;
        assert_eq!(diff.x(), 3.0);
        assert_eq!(diff.y(), 4.0);
        assert_eq!(diff.z(), 5.0);
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
        assert_eq!(p[0], 10.0);
        assert_eq!(p[1], 20.0);
        assert_eq!(p[2], 30.0);
    }

    #[test]
    #[should_panic]
    fn point3_indexing_panics_on_out_of_bounds() {
        let p = Point3::new(1.0, 2.0, 3.0);
        let _ = p[3]; // Should panic
    }

    // General trait tests
    #[test]
    fn points_default_works() {
        let p2 = Point2::default();
        assert_eq!(p2.x(), 0.0);
        assert_eq!(p2.y(), 0.0);

        let p3 = Point3::default();
        assert_eq!(p3.x(), 0.0);
        assert_eq!(p3.y(), 0.0);
        assert_eq!(p3.z(), 0.0);
    }

    #[test]
    fn points_are_copyable() {
        let p2 = Point2::new(1.0, 2.0);
        let p2_copy = p2;
        assert_eq!(p2.x(), p2_copy.x());

        let p3 = Point3::new(1.0, 2.0, 3.0);
        let p3_copy = p3;
        assert_eq!(p3.x(), p3_copy.x());
    }

    #[test]
    fn points_support_equality() {
        let p1 = Point2::new(1.0, 2.0);
        let p2 = Point2::new(1.0, 2.0);
        let p3 = Point2::new(2.0, 1.0);
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);

        let q1 = Point3::new(1.0, 2.0, 3.0);
        let q2 = Point3::new(1.0, 2.0, 3.0);
        let q3 = Point3::new(3.0, 2.0, 1.0);
        assert_eq!(q1, q2);
        assert_ne!(q1, q3);
    }

    #[test]
    fn points_are_debuggable() {
        let p2 = Point2::new(1.0, 2.0);
        let debug_str = format!("{:?}", p2);
        assert!(debug_str.contains("Point2"));

        let p3 = Point3::new(1.0, 2.0, 3.0);
        let debug_str = format!("{:?}", p3);
        assert!(debug_str.contains("Point3"));
    }

    #[test]
    fn point_operations_with_different_reference_combinations_work() {
        let p2 = Point2::new(1.0, 2.0);
        let v2 = Vector2::new(3.0, 4.0);

        // Test all combinations of reference/owned for binary operations
        let _result1 = &p2 + &v2; // ref + ref
        let _result2 = &p2 + v2; // ref + owned
        let _result3 = p2 + &v2; // owned + ref
        let _result4 = p2 + v2; // owned + owned

        // Recreate since they were moved
        let p2 = Point2::new(1.0, 2.0);
        let _result5 = 2.0 * &p2; // scalar * ref
        let _result6 = 2.0 * p2; // scalar * owned

        let p2 = Point2::new(1.0, 2.0);
        let _result7 = &p2 * 2.0; // ref * scalar
        let _result8 = p2 * 2.0; // owned * scalar
    }

    #[test]
    fn point_arithmetic_maintains_precision() {
        let p = Point3::new(0.1, 0.2, 0.3);
        let doubled = &p * 2.0;
        let halved = &doubled / 2.0;

        assert_abs_diff_eq!(halved.x(), p.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.y(), p.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.z(), p.z(), epsilon = EPSILON);
    }

    #[test]
    fn point_vector_point_roundtrip_preserves_data() {
        let original_p2 = Point2::new(1.5, 2.5);
        let as_vector = Vector2::from(original_p2);
        let back_to_point = Point2::from(as_vector);
        assert_eq!(original_p2, back_to_point);

        let original_p3 = Point3::new(1.5, 2.5, 3.5);
        let as_vector = Vector3::from(original_p3);
        let back_to_point = Point3::from(as_vector);
        assert_eq!(original_p3, back_to_point);
    }

    #[test]
    fn point_distance_is_symmetric() {
        let p1 = Point3::new(1.0, 2.0, 3.0);
        let p2 = Point3::new(4.0, 6.0, 8.0);

        let dist1 = Point3::distance_between(&p1, &p2);
        let dist2 = Point3::distance_between(&p2, &p1);
        assert_abs_diff_eq!(dist1, dist2, epsilon = EPSILON);

        let sq_dist1 = Point3::squared_distance_between(&p1, &p2);
        let sq_dist2 = Point3::squared_distance_between(&p2, &p1);
        assert_abs_diff_eq!(sq_dist1, sq_dist2, epsilon = EPSILON);
    }

    #[test]
    fn point_center_is_equidistant() {
        let p1 = Point2::new(0.0, 0.0);
        let p2 = Point2::new(6.0, 8.0);
        let center = Point2::center_of(&p1, &p2);

        let dist_to_p1 = Point2::distance_between(&center, &p1);
        let dist_to_p2 = Point2::distance_between(&center, &p2);
        assert_abs_diff_eq!(dist_to_p1, dist_to_p2, epsilon = EPSILON);
    }
}
