//! Vectors.

use bytemuck::{Pod, Zeroable};
use roc_integration::impl_roc_for_library_provided_primitives;
use std::ops::{Deref, Index, IndexMut};

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Vector2 {
    inner: nalgebra::Vector2<f32>,
}

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Vector3 {
    inner: nalgebra::Vector3<f32>,
}

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Zeroable, Pod)]
pub struct Vector4 {
    inner: nalgebra::Vector4<f32>,
}

#[repr(transparent)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct UnitVector3 {
    inner: nalgebra::UnitVector3<f32>,
}

impl Vector2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            inner: nalgebra::Vector2::new(x, y),
        }
    }

    #[inline]
    pub fn zeros() -> Self {
        Self {
            inner: nalgebra::Vector2::zeros(),
        }
    }

    #[inline]
    pub fn same(value: f32) -> Self {
        Self {
            inner: nalgebra::Vector2::repeat(value),
        }
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
    pub fn normalized(&self) -> Self {
        Self::_wrap(self.inner.normalize())
    }

    #[inline]
    pub fn norm(&self) -> f32 {
        self.inner.norm()
    }

    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.inner.norm_squared()
    }

    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.inner.dot(&other.inner)
    }

    #[inline]
    pub fn component_abs(&self) -> Self {
        Self::_wrap(self.inner.abs())
    }

    #[inline]
    pub fn component_mul(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.component_mul(&other.inner))
    }

    #[inline]
    pub fn component_min(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.inf(&other.inner))
    }

    #[inline]
    pub fn component_max(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.sup(&other.inner))
    }

    #[inline]
    pub fn max_component(&self) -> f32 {
        self.inner.max()
    }

    #[inline]
    pub fn _wrap(inner: nalgebra::Vector2<f32>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn _inner(&self) -> &nalgebra::Vector2<f32> {
        &self.inner
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
    Vector2 {
        inner: a.inner + b.inner,
    }
});

impl_binop!(Sub, sub, Vector2, Vector2, Vector2, |a, b| {
    Vector2 {
        inner: a.inner - b.inner,
    }
});

impl_binop!(Mul, mul, Vector2, f32, Vector2, |a, b| {
    Vector2 {
        inner: a.inner * *b,
    }
});

impl_binop!(Mul, mul, f32, Vector2, Vector2, |a, b| {
    Vector2 {
        inner: *a * b.inner,
    }
});

impl_binop!(Div, div, Vector2, f32, Vector2, |a, b| {
    #[allow(clippy::suspicious_arithmetic_impl)]
    Vector2 {
        inner: a.inner * b.recip(),
    }
});

impl_binop_assign!(AddAssign, add_assign, Vector2, Vector2, |a, b| {
    a.inner.add_assign(b._inner());
});

impl_binop_assign!(SubAssign, sub_assign, Vector2, Vector2, |a, b| {
    a.inner.sub_assign(b._inner());
});

impl_binop_assign!(MulAssign, mul_assign, Vector2, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector2, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector2, Vector2, |val| {
    Vector2 {
        inner: val.inner.neg(),
    }
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
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Vector2, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl Vector3 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            inner: nalgebra::Vector3::new(x, y, z),
        }
    }

    #[inline]
    pub fn zeros() -> Self {
        Self {
            inner: nalgebra::Vector3::zeros(),
        }
    }

    #[inline]
    pub fn same(value: f32) -> Self {
        Self {
            inner: nalgebra::Vector3::repeat(value),
        }
    }

    #[inline]
    pub fn unit_x() -> Self {
        Self {
            inner: nalgebra::Vector3::x(),
        }
    }

    #[inline]
    pub fn unit_y() -> Self {
        Self {
            inner: nalgebra::Vector3::y(),
        }
    }

    #[inline]
    pub fn unit_z() -> Self {
        Self {
            inner: nalgebra::Vector3::z(),
        }
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
    pub fn xy(&self) -> Vector2 {
        Vector2::_wrap(self.inner.xy())
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        Self::_wrap(self.inner.normalize())
    }

    #[inline]
    pub fn norm(&self) -> f32 {
        self.inner.norm()
    }

    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.inner.norm_squared()
    }

    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.inner.dot(&other.inner)
    }

    #[inline]
    pub fn cross(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.cross(&other.inner))
    }

    #[inline]
    pub fn component_abs(&self) -> Self {
        Self::_wrap(self.inner.abs())
    }

    #[inline]
    pub fn component_mul(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.component_mul(&other.inner))
    }

    #[inline]
    pub fn component_min(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.inf(&other.inner))
    }

    #[inline]
    pub fn component_max(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.sup(&other.inner))
    }

    #[inline]
    pub fn max_component(&self) -> f32 {
        self.inner.max()
    }

    #[inline]
    pub fn _wrap(inner: nalgebra::Vector3<f32>) -> Self {
        Self { inner }
    }

    #[inline]
    pub const fn _inner(&self) -> &nalgebra::Vector3<f32> {
        &self.inner
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
    Vector3 {
        inner: a.inner + b.inner,
    }
});

impl_binop!(Sub, sub, Vector3, Vector3, Vector3, |a, b| {
    Vector3 {
        inner: a.inner - b.inner,
    }
});

impl_binop!(Mul, mul, Vector3, f32, Vector3, |a, b| {
    Vector3 {
        inner: a.inner * *b,
    }
});

impl_binop!(Mul, mul, f32, Vector3, Vector3, |a, b| {
    Vector3 {
        inner: *a * b.inner,
    }
});

impl_binop!(Div, div, Vector3, f32, Vector3, |a, b| {
    #[allow(clippy::suspicious_arithmetic_impl)]
    Vector3 {
        inner: a.inner * b.recip(),
    }
});

impl_binop_assign!(AddAssign, add_assign, Vector3, Vector3, |a, b| {
    a.inner.add_assign(b._inner());
});

impl_binop_assign!(SubAssign, sub_assign, Vector3, Vector3, |a, b| {
    a.inner.sub_assign(b._inner());
});

impl_binop_assign!(MulAssign, mul_assign, Vector3, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector3, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector3, Vector3, |val| {
    Vector3 {
        inner: val.inner.neg(),
    }
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
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Vector3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl Vector4 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            inner: nalgebra::Vector4::new(x, y, z, w),
        }
    }

    #[inline]
    pub fn zeros() -> Self {
        Self {
            inner: nalgebra::Vector4::zeros(),
        }
    }

    #[inline]
    pub fn same(value: f32) -> Self {
        Self {
            inner: nalgebra::Vector4::repeat(value),
        }
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
    pub fn w(&self) -> f32 {
        self.inner.w
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
    pub fn w_mut(&mut self) -> &mut f32 {
        &mut self.inner.w
    }

    #[inline]
    pub fn xyz(&self) -> Vector3 {
        Vector3::_wrap(self.inner.xyz())
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        Self::_wrap(self.inner.normalize())
    }

    #[inline]
    pub fn norm(&self) -> f32 {
        self.inner.norm()
    }

    #[inline]
    pub fn norm_squared(&self) -> f32 {
        self.inner.norm_squared()
    }

    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.inner.dot(&other.inner)
    }

    #[inline]
    pub fn component_abs(&self) -> Self {
        Self::_wrap(self.inner.abs())
    }

    #[inline]
    pub fn component_mul(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.component_mul(&other.inner))
    }

    #[inline]
    pub fn component_min(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.inf(&other.inner))
    }

    #[inline]
    pub fn component_max(&self, other: &Self) -> Self {
        Self::_wrap(self.inner.sup(&other.inner))
    }

    #[inline]
    pub fn max_component(&self) -> f32 {
        self.inner.max()
    }

    #[inline]
    pub fn _wrap(inner: nalgebra::Vector4<f32>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn _inner(&self) -> &nalgebra::Vector4<f32> {
        &self.inner
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
    Vector4 {
        inner: a.inner + b.inner,
    }
});

impl_binop!(Sub, sub, Vector4, Vector4, Vector4, |a, b| {
    Vector4 {
        inner: a.inner - b.inner,
    }
});

impl_binop!(Mul, mul, Vector4, f32, Vector4, |a, b| {
    Vector4 {
        inner: a.inner * *b,
    }
});

impl_binop!(Mul, mul, f32, Vector4, Vector4, |a, b| {
    Vector4 {
        inner: *a * b.inner,
    }
});

impl_binop!(Div, div, Vector4, f32, Vector4, |a, b| {
    #[allow(clippy::suspicious_arithmetic_impl)]
    Vector4 {
        inner: a.inner * b.recip(),
    }
});

impl_binop_assign!(AddAssign, add_assign, Vector4, Vector4, |a, b| {
    a.inner.add_assign(b._inner());
});

impl_binop_assign!(SubAssign, sub_assign, Vector4, Vector4, |a, b| {
    a.inner.sub_assign(b._inner());
});

impl_binop_assign!(MulAssign, mul_assign, Vector4, f32, |a, b| {
    a.inner.mul_assign(*b);
});

impl_binop_assign!(DivAssign, div_assign, Vector4, f32, |a, b| {
    a.inner.div_assign(*b);
});

impl_unary_op!(Neg, neg, Vector4, Vector4, |val| {
    Vector4 {
        inner: val.inner.neg(),
    }
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
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(Vector4, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl UnitVector3 {
    #[inline]
    pub fn unit_x() -> Self {
        Self {
            inner: nalgebra::Vector3::x_axis(),
        }
    }

    #[inline]
    pub fn unit_y() -> Self {
        Self {
            inner: nalgebra::Vector3::y_axis(),
        }
    }

    #[inline]
    pub fn unit_z() -> Self {
        Self {
            inner: nalgebra::Vector3::z_axis(),
        }
    }

    #[inline]
    pub fn normalized_from(vector: Vector3) -> Self {
        Self {
            inner: nalgebra::UnitVector3::new_normalize(vector.inner),
        }
    }

    #[inline]
    pub fn normalized_from_if_above(vector: Vector3, min_norm: f32) -> Option<Self> {
        nalgebra::UnitVector3::try_new(vector.inner, min_norm).map(|inner| Self { inner })
    }

    #[inline]
    pub fn normalized_from_and_norm(vector: Vector3) -> (Self, f32) {
        let (inner, norm) = nalgebra::UnitVector3::new_and_get(vector.inner);
        (Self { inner }, norm)
    }

    #[inline]
    pub fn normalized_from_and_norm_if_above(
        vector: Vector3,
        min_norm: f32,
    ) -> Option<(Self, f32)> {
        nalgebra::UnitVector3::try_new_and_get(vector.inner, min_norm)
            .map(|(inner, norm)| (Self { inner }, norm))
    }

    #[inline]
    pub const fn unchecked_from(vector: Vector3) -> Self {
        Self {
            inner: nalgebra::UnitVector3::new_unchecked(vector.inner),
        }
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
    pub fn as_vector(&self) -> &Vector3 {
        bytemuck::from_bytes(bytemuck::bytes_of(&self.inner))
    }

    #[inline]
    pub fn _wrap(inner: nalgebra::UnitVector3<f32>) -> Self {
        Self { inner }
    }

    #[inline]
    pub const fn _inner(&self) -> &nalgebra::UnitVector3<f32> {
        &self.inner
    }
}

impl Deref for UnitVector3 {
    type Target = Vector3;

    #[inline]
    fn deref(&self) -> &Self::Target {
        bytemuck::from_bytes(bytemuck::bytes_of(self))
    }
}

impl_binop!(Mul, mul, UnitVector3, f32, Vector3, |a, b| {
    Vector3 {
        inner: a.inner.scale(*b),
    }
});

impl_binop!(Mul, mul, f32, UnitVector3, Vector3, |a, b| {
    Vector3 {
        inner: b.inner.scale(*a),
    }
});

impl_unary_op!(Neg, neg, UnitVector3, UnitVector3, |val| {
    UnitVector3 {
        inner: val.inner.neg(),
    }
});

impl Index<usize> for UnitVector3 {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl_abs_diff_eq!(UnitVector3, |a, b, epsilon| {
    a.inner.abs_diff_eq(&b.inner, epsilon)
});

impl_relative_eq!(UnitVector3, |a, b, epsilon, max_relative| {
    a.inner.relative_eq(&b.inner, epsilon, max_relative)
});

impl_roc_for_library_provided_primitives! {
//  Type           Pkg   Parents  Module       Roc name     Postfix  Precision
    Vector2     => core, None,    Vector2,     Vector2,     None,    PrecisionIrrelevant,
    Vector3     => core, None,    Vector3,     Vector3,     None,    PrecisionIrrelevant,
    Vector4     => core, None,    Vector4,     Vector4,     None,    PrecisionIrrelevant,
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
    fn vector3_cross_product_works() {
        let v1 = Vector3::new(1.0, 0.0, 0.0);
        let v2 = Vector3::new(0.0, 1.0, 0.0);
        let cross = v1.cross(&v2);
        assert_abs_diff_eq!(cross.x(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(cross.z(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3_xy_extraction_works() {
        let v3 = Vector3::new(1.0, 2.0, 3.0);
        let xy = v3.xy();
        assert_eq!(xy.x(), 1.0);
        assert_eq!(xy.y(), 2.0);
    }

    #[test]
    fn vector4_new_works() {
        let v = Vector4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.w(), 4.0);
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
    fn unitvector3_unit_vectors_work() {
        let unit_x = UnitVector3::unit_x();
        assert_eq!(unit_x.x(), 1.0);
        assert_eq!(unit_x.y(), 0.0);
        assert_eq!(unit_x.z(), 0.0);
        assert_abs_diff_eq!(unit_x.norm(), 1.0, epsilon = EPSILON);

        let unit_y = UnitVector3::unit_y();
        assert_eq!(unit_y.x(), 0.0);
        assert_eq!(unit_y.y(), 1.0);
        assert_eq!(unit_y.z(), 0.0);
        assert_abs_diff_eq!(unit_y.norm(), 1.0, epsilon = EPSILON);

        let unit_z = UnitVector3::unit_z();
        assert_eq!(unit_z.x(), 0.0);
        assert_eq!(unit_z.y(), 0.0);
        assert_eq!(unit_z.z(), 1.0);
        assert_abs_diff_eq!(unit_z.norm(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3_normalized_from_works() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let unit = UnitVector3::normalized_from(v);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.x(), 0.6, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.y(), 0.8, epsilon = EPSILON);
        assert_abs_diff_eq!(unit.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3_normalized_from_if_above_works() {
        let v_large = Vector3::new(3.0, 4.0, 0.0);
        let unit_large = UnitVector3::normalized_from_if_above(v_large, 1.0);
        assert!(unit_large.is_some());
        let unit = unit_large.unwrap();
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);

        let v_small = Vector3::new(0.1, 0.1, 0.0);
        let unit_small = UnitVector3::normalized_from_if_above(v_small, 1.0);
        assert!(unit_small.is_none());
    }

    #[test]
    fn unitvector3_normalized_from_and_norm_works() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let (unit, norm) = UnitVector3::normalized_from_and_norm(v);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(norm, 5.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3_normalized_from_and_norm_if_above_works() {
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
    fn unitvector3_unchecked_from_works() {
        let v = Vector3::new(1.0, 0.0, 0.0); // Already normalized
        let unit = UnitVector3::unchecked_from(v);
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
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
    fn unitvector3_deref_to_vector3_works() {
        let unit = UnitVector3::unit_x();
        // Test that UnitVector3 can be used as Vector3 through Deref
        assert_eq!(unit.x(), 1.0);
        assert_eq!(unit.y(), 0.0);
        assert_eq!(unit.z(), 0.0);
        assert_abs_diff_eq!(unit.norm(), 1.0, epsilon = EPSILON);
    }

    #[test]
    fn unitvector3_indexing_works() {
        let unit = UnitVector3::unit_y();
        assert_eq!(unit[0], 0.0);
        assert_eq!(unit[1], 1.0);
        assert_eq!(unit[2], 0.0);
    }

    #[test]
    fn unitvector3_arithmetic_with_scalar_works() {
        let unit = UnitVector3::unit_x();
        let scaled = &unit * 2.0;
        assert_eq!(scaled.x(), 2.0);
        assert_eq!(scaled.y(), 0.0);
        assert_eq!(scaled.z(), 0.0);
    }

    // Additional Vector3 tests for complete coverage
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
    fn vector3_norm_calculations_work() {
        let v = Vector3::new(1.0, 2.0, 2.0);
        assert_abs_diff_eq!(v.norm(), 3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 9.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3_normalized_gives_unit_vector() {
        let v = Vector3::new(2.0, 0.0, 0.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.z(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector3_dot_product_works() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);
        assert_abs_diff_eq!(v1.dot(&v2), 32.0, epsilon = EPSILON); // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn vector3_component_operations_work() {
        let v1 = Vector3::new(-1.0, 2.0, -3.0);
        let v2 = Vector3::new(4.0, -5.0, 6.0);

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
    fn vector3_max_component_returns_largest_element() {
        let v = Vector3::new(1.5, 3.7, 2.1);
        assert_abs_diff_eq!(v.max_component(), 3.7, epsilon = EPSILON);
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

    // Additional Vector4 tests for complete coverage
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
    fn vector4_norm_calculations_work() {
        let v = Vector4::new(1.0, 2.0, 2.0, 0.0);
        assert_abs_diff_eq!(v.norm(), 3.0, epsilon = EPSILON);
        assert_abs_diff_eq!(v.norm_squared(), 9.0, epsilon = EPSILON);
    }

    #[test]
    fn vector4_normalized_gives_unit_vector() {
        let v = Vector4::new(2.0, 0.0, 0.0, 0.0);
        let normalized = v.normalized();
        assert_abs_diff_eq!(normalized.norm(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.x(), 1.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.y(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.z(), 0.0, epsilon = EPSILON);
        assert_abs_diff_eq!(normalized.w(), 0.0, epsilon = EPSILON);
    }

    #[test]
    fn vector4_dot_product_works() {
        let v1 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vector4::new(5.0, 6.0, 7.0, 8.0);
        assert_abs_diff_eq!(v1.dot(&v2), 70.0, epsilon = EPSILON); // 1*5 + 2*6 + 3*7 + 4*8 = 70
    }

    #[test]
    fn vector4_component_operations_work() {
        let v1 = Vector4::new(-1.0, 2.0, -3.0, 4.0);
        let v2 = Vector4::new(5.0, -6.0, 7.0, -8.0);

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
    fn vector4_max_component_returns_largest_element() {
        let v = Vector4::new(1.5, 3.7, 2.1, 0.8);
        assert_abs_diff_eq!(v.max_component(), 3.7, epsilon = EPSILON);
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

    // Edge cases and boundary conditions
    #[test]
    #[should_panic]
    fn vector_indexing_panics_on_out_of_bounds() {
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
    fn vector4_indexing_panics_on_out_of_bounds() {
        let v = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let _ = v[4]; // Should panic
    }

    #[test]
    fn normalized_zero_vector_returns_nan() {
        // nalgebra returns NaN when normalizing a zero vector
        let zero = Vector3::zeros();
        let normalized = zero.normalized();
        assert!(normalized.x().is_nan());
        assert!(normalized.y().is_nan());
        assert!(normalized.z().is_nan());
    }

    #[test]
    fn unitvector3_from_zero_vector_returns_nan() {
        // nalgebra returns NaN when normalizing a zero vector
        let zero = Vector3::zeros();
        let unit = UnitVector3::normalized_from(zero);
        assert!(unit.norm().is_nan());
    }

    #[test]
    fn vector_operations_with_different_reference_combinations_work() {
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
    }

    #[test]
    fn vector_arithmetic_maintains_precision() {
        let v = Vector3::new(0.1, 0.2, 0.3);
        let doubled = &v * 2.0;
        let halved = &doubled / 2.0;

        assert_abs_diff_eq!(halved.x(), v.x(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.y(), v.y(), epsilon = EPSILON);
        assert_abs_diff_eq!(halved.z(), v.z(), epsilon = EPSILON);
    }

    #[test]
    fn unitvector3_maintains_unit_length_through_operations() {
        let unit = UnitVector3::normalized_from(Vector3::new(1.0, 2.0, 3.0));
        let scaled_back = (&unit * 5.0) / 5.0;

        assert_abs_diff_eq!(scaled_back.norm(), 1.0, epsilon = EPSILON);
    }
}
