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
    fn from(vector: Vector2) -> Self {
        Self {
            inner: (*vector._inner()).into(),
        }
    }
}

impl From<Point2> for Vector2 {
    fn from(point: Point2) -> Self {
        Vector2::_wrap(point.inner.coords)
    }
}

impl From<[f32; 2]> for Point2 {
    fn from([x, y]: [f32; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<Point2> for [f32; 2] {
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

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
}

impl IndexMut<usize> for Point2 {
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
    fn from(vector: Vector3) -> Self {
        Self {
            inner: (*vector._inner()).into(),
        }
    }
}

impl From<Point3> for Vector3 {
    fn from(point: Point3) -> Self {
        Vector3::_wrap(point.inner.coords)
    }
}

impl From<[f32; 3]> for Point3 {
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl From<Point3> for [f32; 3] {
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
