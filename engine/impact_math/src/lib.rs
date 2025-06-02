//! Math utilities.

#[macro_use]
mod macros;

mod halton;
mod hash;
mod num;

pub use halton::HaltonSequence;
pub use hash::{
    ConstStringHash64, Hash32, Hash64, StringHash32, StringHash64, compute_hash_64_of_two_hash_64,
    compute_hash_str_32, compute_hash_str_64,
};
pub use num::Float;
