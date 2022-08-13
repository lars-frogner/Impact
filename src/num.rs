//! Numbers and numerics.

use nalgebra as na;
use num_traits as nt;

/// Gathers traits useful for working with generic floating point types.
pub trait Float: Copy + nt::FloatConst + nt::FromPrimitive + na::RealField + na::Scalar {
    const ZERO: Self;
    const ONE: Self;
    const NEG_ONE: Self;
}

macro_rules! impl_float {
    ($f:ty) => {
        impl Float for $f {
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const NEG_ONE: Self = -1.0;
        }
    };
}

impl_float!(f32);
impl_float!(f64);
