//! Numbers and numerics.

use nalgebra as na;
use num_traits as nt;

/// Gathers traits useful for working with generic floating point types.
pub trait Float: Copy + nt::FloatConst + nt::FromPrimitive + na::RealField + na::Scalar {
    const ZERO: Self;
    const ONE: Self;
    const NEG_ONE: Self;
    const TWO: Self;
    const THREE: Self;
    const FOUR: Self;
    const EIGHT: Self;
    const ONE_HALF: Self;
    const ONE_THIRD: Self;
    const ONE_QUARTER: Self;
    const FRAC_1_SQRT_2: Self;
    const NEG_FRAC_1_SQRT_2: Self;
    const PI: Self;
    const TWO_PI: Self;
    const FRAC_PI_2: Self;
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
            const THREE: Self = 3.0;
            const FOUR: Self = 4.0;
            const EIGHT: Self = 8.0;
            const ONE_HALF: Self = 0.5;
            const ONE_THIRD: Self = 1.0 / 3.0;
            const ONE_QUARTER: Self = 0.25;
            const FRAC_1_SQRT_2: Self = std::$f::consts::FRAC_1_SQRT_2;
            const NEG_FRAC_1_SQRT_2: Self = -std::$f::consts::FRAC_1_SQRT_2;
            const PI: Self = std::$f::consts::PI;
            const TWO_PI: Self = 2.0 * std::$f::consts::PI;
            const FRAC_PI_2: Self = std::$f::consts::FRAC_PI_2;
            const MIN: Self = Self::MIN;
            const MAX: Self = Self::MAX;
        }
    };
}

impl_float!(f32);
impl_float!(f64);
