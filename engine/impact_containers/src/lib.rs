//! Containers and data structures.

mod aligned_byte_vec;
mod generational_reusing_vec;
mod key_index_mapper;
mod tracking;

pub use aligned_byte_vec::{AlignedByteVec, Alignment};
pub use generational_reusing_vec::{GenerationalIdx, GenerationalReusingVec};
pub use key_index_mapper::KeyIndexMapper;
pub use tracking::{CollectionChange, CollectionChangeTracker, EntityChangeTracker};
