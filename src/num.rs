//! Numbers and numerics.

use nalgebra as na;
use num_traits as nt;

pub trait Float:
    nt::Float + nt::FloatConst + nt::FromPrimitive + na::RealField + na::Scalar
{
}

impl Float for f32 {}
impl Float for f64 {}

#[macro_export]
macro_rules! float_from {
    ($type_param:ident, $value:expr) => {
        $type_param::from_f64($value).unwrap()
    };
}
