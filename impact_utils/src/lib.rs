//! General utilities.

#[macro_use]
mod macros;

mod aligned_byte_vec;
mod generational_reusing_vec;
mod halton;
mod hash;
mod key_index_mapper;

pub use aligned_byte_vec::{AlignedByteVec, Alignment};
pub use generational_reusing_vec::{GenerationalIdx, GenerationalReusingVec};
pub use halton::HaltonSequence;
pub use hash::{
    ConstStringHash64, Hash32, Hash64, StringHash32, StringHash64, compute_hash_64_of_two_hash_64,
    compute_hash_str_32, compute_hash_str_64,
};
pub use key_index_mapper::KeyIndexMapper;
