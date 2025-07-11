//! Math utilities.

#[macro_use]
mod macros;

mod angle;
mod bounds;
mod halton;
mod hash;
mod num;

pub use angle::{Angle, Degrees, Radians};
pub use bounds::{Bounds, InclusiveBounds, UpperExclusiveBounds};
pub use halton::HaltonSequence;
pub use hash::{
    ConstStringHash64, Hash32, Hash64, StringHash32, StringHash64, compute_hash_64_of_two_hash_64,
};
pub use num::Float;
