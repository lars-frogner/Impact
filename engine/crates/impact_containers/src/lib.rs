//! Containers and data structures.

mod aligned_byte_vec;
mod bit_vector;
mod key_index_mapper;
mod slot_map;
mod tracking;

pub use aligned_byte_vec::{AlignedByteVec, Alignment};
pub use bit_vector::BitVector;
pub use key_index_mapper::KeyIndexMapper;
pub use slot_map::{SlotKey, SlotMap};
pub use tracking::{CollectionChange, CollectionChangeTracker, EntityChangeTracker};

pub use rustc_hash::FxBuildHasher as RandomState;
pub use rustc_hash::FxHashMap as HashMap;
pub use rustc_hash::FxHashSet as HashSet;
pub use rustc_hash::FxHasher as DefaultHasher;

pub type NoHashMap<K, V> = std::collections::HashMap<K, V, nohash_hasher::BuildNoHashHasher<K>>;
pub type NoHashSet<K> = std::collections::HashSet<K, nohash_hasher::BuildNoHashHasher<K>>;
pub type NoHashKeyIndexMapper<K> = KeyIndexMapper<K, nohash_hasher::BuildNoHashHasher<K>>;

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
pub type IndexSet<K> = indexmap::IndexSet<K, rustc_hash::FxBuildHasher>;
