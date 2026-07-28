//! Generation of signed distance fields.

pub mod atomic;
pub mod meta;

pub use atomic::*;

#[derive(Clone, Copy, Debug)]
pub struct Smoothness {
    smoothness: f32,
    quarter_inv_smoothness: f32,
}

impl Smoothness {
    #[inline]
    pub fn new(smoothness: f32) -> Self {
        Self {
            smoothness,
            quarter_inv_smoothness: 0.25 / smoothness,
        }
    }

    #[inline]
    pub fn get(&self) -> f32 {
        self.smoothness
    }

    #[inline]
    pub fn scaled(&self, scale: f32) -> Self {
        Self::new(self.smoothness * scale)
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.smoothness == 0.0
    }
}

impl From<f32> for Smoothness {
    #[inline]
    fn from(smoothness: f32) -> Self {
        Self::new(smoothness)
    }
}

#[inline]
pub fn sdf_union(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    if smoothness.is_zero() {
        hard_sdf_union(distance_1, distance_2)
    } else {
        smooth_sdf_union(distance_1, distance_2, smoothness)
    }
}

#[inline]
pub fn sdf_subtraction(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    if smoothness.is_zero() {
        hard_sdf_subtraction(distance_1, distance_2)
    } else {
        smooth_sdf_subtraction(distance_1, distance_2, smoothness)
    }
}

#[inline]
pub fn sdf_intersection(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    if smoothness.is_zero() {
        hard_sdf_intersection(distance_1, distance_2)
    } else {
        smooth_sdf_intersection(distance_1, distance_2, smoothness)
    }
}

#[inline]
pub fn hard_sdf_union(distance_1: f32, distance_2: f32) -> f32 {
    f32::min(distance_1, distance_2)
}

#[inline]
pub fn hard_sdf_subtraction(distance_1: f32, distance_2: f32) -> f32 {
    f32::max(distance_1, -distance_2)
}

#[inline]
pub fn hard_sdf_intersection(distance_1: f32, distance_2: f32) -> f32 {
    f32::max(distance_1, distance_2)
}

#[inline]
pub fn smooth_sdf_union(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    let h = (smoothness.get() - (distance_1 - distance_2).abs()).max(0.0);
    distance_1.min(distance_2) - (h * h) * smoothness.quarter_inv_smoothness
}

#[inline]
pub fn smooth_sdf_subtraction(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    -smooth_sdf_union(-distance_1, distance_2, smoothness)
}

#[inline]
pub fn smooth_sdf_intersection(distance_1: f32, distance_2: f32, smoothness: Smoothness) -> f32 {
    -smooth_sdf_union(-distance_1, -distance_2, smoothness)
}
