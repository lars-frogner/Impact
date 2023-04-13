//! Numbers and numerics.

use nalgebra as na;
use num_traits as nt;

/// Gathers traits useful for working with generic floating point types.
pub trait Float: Copy + nt::FloatConst + nt::FromPrimitive + na::RealField + na::Scalar {
    const ZERO: Self;
    const ONE: Self;
    const NEG_ONE: Self;
    const TWO: Self;
    const ONE_HALF: Self;
    const FRAC_1_SQRT_2: Self;
    const NEG_FRAC_1_SQRT_2: Self;
    const PI: Self;
    const TWO_PI: Self;
    const MIN: Self;
    const MAX: Self;
}

macro_rules! impl_float {
    ($f:tt) => {
        impl Float for $f {
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const NEG_ONE: Self = -1.0;
            const TWO: Self = 2.0;
            const ONE_HALF: Self = 0.5;
            const FRAC_1_SQRT_2: Self = std::$f::consts::FRAC_1_SQRT_2;
            const NEG_FRAC_1_SQRT_2: Self = -std::$f::consts::FRAC_1_SQRT_2;
            const PI: Self = std::$f::consts::PI;
            const TWO_PI: Self = 2.0 * std::$f::consts::PI;
            const MIN: Self = Self::MIN;
            const MAX: Self = Self::MAX;
        }
    };
}

impl_float!(f32);
impl_float!(f64);
