//! Containers and data structures.

pub mod aligned_byte_vec;
pub mod bit_vector;
pub mod fixed_queue;
pub mod key_index_mapper;
pub mod slot_map;
pub mod tracking;

pub use aligned_byte_vec::{AlignedByteVec, Alignment};
pub use bit_vector::BitVector;
pub use fixed_queue::FixedQueue;
pub use key_index_mapper::KeyIndexMapper;
pub use slot_map::{SlotKey, SlotMap};

pub use hashbrown::hash_map;
pub use hashbrown::hash_set;

pub use rustc_hash::FxBuildHasher as RandomState;
pub use rustc_hash::FxHasher as DefaultHasher;

pub use nohash_hasher;

use impact_alloc::Global;

pub type HashMap<K, V, A = Global> = hashbrown::HashMap<K, V, rustc_hash::FxBuildHasher, A>;
pub type HashSet<T, A = Global> = hashbrown::HashSet<T, rustc_hash::FxBuildHasher, A>;

pub type NoHashMap<K, V, A = Global> =
    hashbrown::HashMap<K, V, nohash_hasher::BuildNoHashHasher<K>, A>;
pub type NoHashSet<K, A = Global> = hashbrown::HashSet<K, nohash_hasher::BuildNoHashHasher<K>, A>;

pub type NoHashKeyIndexMapper<K, A = Global> =
    KeyIndexMapper<K, nohash_hasher::BuildNoHashHasher<K>, A>;

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
pub type IndexSet<K> = indexmap::IndexSet<K, rustc_hash::FxBuildHasher>;
