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
    fn from([x, y]: [f32; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<Vector2> for [f32; 2] {
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

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector2 {
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
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Vector3> for [f32; 3] {
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

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector3 {
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
    fn from([x, y, z, w]: [f32; 4]) -> Self {
        Self::new(x, y, z, w)
    }
}

impl From<Vector4> for [f32; 4] {
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

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Vector4 {
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
