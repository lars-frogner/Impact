//! Numbers and numerics.

use nalgebra as na;
use num_traits as nt;

/// Gathers traits useful for working with generic floating point types.
pub trait Float: Copy + nt::FloatConst + nt::FromPrimitive + na::RealField + na::Scalar {}

impl Float for f32 {}
impl Float for f64 {}
