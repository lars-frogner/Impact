//! Numbers and numerics.

#![allow(clippy::excessive_precision)]

use num_traits as nt;

/// Gathers traits useful for working with generic floating point types.
pub trait Float:
    nt::Float + nt::FromPrimitive + nt::ToPrimitive + approx::AbsDiffEq + approx::RelativeEq
{
    const ZERO: Self;
    const ONE: Self;
    const NEG_ONE: Self;
    const TWO: Self;
    const THREE: Self;
    const FOUR: Self;
    const FIVE: Self;
    const EIGHT: Self;
    const ONE_HALF: Self;
    const ONE_THIRD: Self;
    const ONE_FOURTH: Self;
    const ONE_FIFTH: Self;
    const ONE_SIXTH: Self;
    const SQRT_2: Self;
    const FRAC_1_SQRT_2: Self;
    const NEG_FRAC_1_SQRT_2: Self;
    const SQRT_3: Self;
    const FRAC_1_SQRT_3: Self;
    const PI: Self;
    const TWO_PI: Self;
    const FRAC_PI_2: Self;
    const FRAC_1_PI: Self;
    const MIN: Self;
    const MAX: Self;
    const INFINITY: Self;
    const NEG_INFINITY: Self;
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
            const FIVE: Self = 5.0;
            const EIGHT: Self = 8.0;
            const ONE_HALF: Self = 0.5;
            const ONE_THIRD: Self = 1.0 / 3.0;
            const ONE_FOURTH: Self = 0.25;
            const ONE_FIFTH: Self = 0.2;
            const ONE_SIXTH: Self = 1.0 / 6.0;
            const SQRT_2: Self = std::$f::consts::SQRT_2;
            const FRAC_1_SQRT_2: Self = std::$f::consts::FRAC_1_SQRT_2;
            const NEG_FRAC_1_SQRT_2: Self = -std::$f::consts::FRAC_1_SQRT_2;
            const SQRT_3: Self = 1.732050807568877293527446341505872367;
            const FRAC_1_SQRT_3: Self = 0.577350269189625764509148780501957456;
            const PI: Self = std::$f::consts::PI;
            const TWO_PI: Self = 2.0 * std::$f::consts::PI;
            const FRAC_PI_2: Self = std::$f::consts::FRAC_PI_2;
            const FRAC_1_PI: Self = std::$f::consts::FRAC_1_PI;
            const MIN: Self = Self::MIN;
            const MAX: Self = Self::MAX;
            const INFINITY: Self = Self::INFINITY;
            const NEG_INFINITY: Self = Self::NEG_INFINITY;
        }
    };
}

impl_float!(f32);
impl_float!(f64);
