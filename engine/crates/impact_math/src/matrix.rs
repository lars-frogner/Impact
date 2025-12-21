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
