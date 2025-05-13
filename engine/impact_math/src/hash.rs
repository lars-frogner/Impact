//! Hash-related utilities.

use bytemuck::{Pod, Zeroable};
use lazy_static::lazy_static;
use roc_codegen::roc;
use std::{
    cmp,
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    sync::Mutex,
};

/// A 32-bit hash.
#[roc(
    category = "primitive",
    package = "core",
    module = "Hashing",
    postfix = "_hash_32"
)]
#[repr(transparent)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct Hash32(u32);

/// A 64-bit hash.
#[roc(
    category = "primitive",
    package = "core",
    module = "Hashing",
    postfix = "_hash_64"
)]
#[repr(transparent)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct Hash64(u64);

/// A 32-bit hash of a string.
///
/// This object stores the string in a global registry and can
/// be formatted into it by means of the [`Display`](fmt::Display)
/// trait.
#[roc(
    category = "primitive",
    package = "core",
    module = "Hashing",
    postfix = "_string_hash_32"
)]
#[repr(transparent)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct StringHash32(Hash32);

/// A 64-bit hash of a string.
///
/// This object stores the string in a global registry and can
/// be formatted into it by means of the [`Display`](fmt::Display)
/// trait.
#[roc(
    category = "primitive",
    package = "core",
    module = "Hashing",
    postfix = "_string_hash_64"
)]
#[repr(transparent)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct StringHash64(Hash64);

/// A 64-bit hash of a string literal. Can be constructed at
/// compile time.
///
/// This object remembers the original string and can be
/// formatted into it by means of the [`Display`](fmt::Display)
/// trait.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ConstStringHash64 {
    hash: Hash64,
    string: &'static str,
}

lazy_static! {
    static ref STRING_HASH_32_REGISTRY: Mutex<HashMap<Hash32, String>> = Mutex::new(HashMap::new());
    static ref STRING_HASH_64_REGISTRY: Mutex<HashMap<Hash64, String>> = Mutex::new(HashMap::new());
}

impl From<Hash32> for u32 {
    fn from(hash: Hash32) -> Self {
        hash.0
    }
}

impl From<Hash64> for u64 {
    fn from(hash: Hash64) -> Self {
        hash.0
    }
}

impl StringHash32 {
    /// Creates a new [`StringHash32`] for the given string.
    ///
    /// The string and associated hash are inserted into a global
    /// registry so that the string can be looked up.
    ///
    /// # Concurrency
    /// The method has to temporarily acquire a lock on the global
    /// string registry in order to record the hash and string pair.
    pub fn new<S: ToString>(string: S) -> Self {
        let string = string.to_string();
        let hash = compute_hash_str_32(&string);
        Self::new_with_hash(string, hash)
    }

    /// Creates a new [`StringHash32`] for the given string with the
    /// given precomputed hash.
    ///
    /// The reference to the string literal is stored together
    /// with the hash so that it can be retrieved.
    ///
    /// # Concurrency
    /// The method has to temporarily acquire a lock on the global
    /// string registry in order to record the hash and string pair.
    pub fn new_with_hash<S: ToString>(string: S, hash: Hash32) -> Self {
        STRING_HASH_32_REGISTRY
            .lock()
            .unwrap()
            .entry(hash)
            .or_insert_with(|| string.to_string());
        Self(hash)
    }

    /// The 32-bit hash value.
    pub fn hash(&self) -> Hash32 {
        self.0
    }
}

impl StringHash64 {
    /// Creates a new [`StringHash64`] for the given string.
    ///
    /// The string and associated hash are inserted into a global
    /// registry so that the string can be looked up.
    ///
    /// # Concurrency
    /// The method has to temporarily acquire a lock on the global
    /// string registry in order to record the hash and string pair.
    pub fn new<S: ToString>(string: S) -> Self {
        let string = string.to_string();
        let hash = compute_hash_str_64(&string);
        Self::new_with_hash(string, hash)
    }

    /// Creates a new [`StringHash64`] for the given string with the
    /// given precomputed hash.
    ///
    /// The reference to the string literal is stored together
    /// with the hash so that it can be retrieved.
    ///
    /// # Concurrency
    /// The method has to temporarily acquire a lock on the global
    /// string registry in order to record the hash and string pair.
    pub fn new_with_hash<S: ToString>(string: S, hash: Hash64) -> Self {
        STRING_HASH_64_REGISTRY
            .lock()
            .unwrap()
            .entry(hash)
            .or_insert_with(|| string.to_string());
        Self(hash)
    }

    /// The 64-bit hash value.
    pub fn hash(&self) -> Hash64 {
        self.0
    }
}

impl ConstStringHash64 {
    /// Creates a hash of the given string literal. This method
    /// is evaluated at compile time.
    ///
    /// The reference to the string literal is stored together
    /// with the hash so that it can be retrieved.
    pub const fn new(string: &'static str) -> Self {
        Self {
            hash: compute_hash_str_64(string),
            string,
        }
    }

    /// Returns the string literal.
    pub const fn string(&self) -> &'static str {
        self.string
    }

    /// Unwraps the string hash to obtain the underlying [`Hash64`].
    pub const fn into_hash(self) -> Hash64 {
        let Self { hash, string: _ } = self;
        hash
    }
}

/// Computes a 64-bit hash of the given string literal.
pub const fn compute_hash_str_64(string: &str) -> Hash64 {
    Hash64(const_fnv1a_hash::fnv1a_hash_str_64(string))
}

/// Computes a 32-bit hash of the given string literal.
pub const fn compute_hash_str_32(string: &str) -> Hash32 {
    Hash32(const_fnv1a_hash::fnv1a_hash_str_32(string))
}

/// Computes a 64-bit hash of the concatenated bytes of the
/// given pair of 64-bit hashes.
pub const fn compute_hash_64_of_two_hash_64(hash_1: Hash64, hash_2: Hash64) -> Hash64 {
    let b1 = &hash_1.0.to_le_bytes();
    let b2 = &hash_2.0.to_le_bytes();
    Hash64(const_fnv1a_hash::fnv1a_hash_64(
        &[
            b1[0], b1[1], b1[2], b1[3], b1[4], b1[5], b1[6], b1[7], b2[0], b2[1], b2[2], b2[3],
            b2[4], b2[5], b2[6], b2[7],
        ],
        None,
    ))
}

impl fmt::Display for Hash32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Hash64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for StringHash32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(string) = STRING_HASH_32_REGISTRY.lock().unwrap().get(&self.0) {
            write!(f, "{}", string)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl fmt::Display for StringHash64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(string) = STRING_HASH_64_REGISTRY.lock().unwrap().get(&self.0) {
            write!(f, "{}", string)
        } else {
            write!(f, "{}", self.0)
        }
    }
}

impl fmt::Display for ConstStringHash64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl AsRef<str> for ConstStringHash64 {
    fn as_ref(&self) -> &str {
        self.string
    }
}

impl Ord for ConstStringHash64 {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl PartialOrd for ConstStringHash64 {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Disabling this error because the requirement for `Hash`,
// `k1 == k2 -> hash(k1) == hash(k2)`, is still upheld
// even though we only hash one of the fields
#[allow(clippy::derived_hash_with_manual_eq)]
impl Hash for ConstStringHash64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_hash_remembers_string() {
        let string = "Foo-bar";
        let hash = StringHash64::new(string);
        assert_eq!(string, &hash.to_string());
    }

    #[test]
    fn const_string_hash_remembers_string() {
        let string = "Foo-bar";
        let hash = ConstStringHash64::new(string);
        assert_eq!(string, hash.to_string());
    }

    #[test]
    fn hash_macro_remembers_string() {
        let string = "Foo-bar".to_string();
        let hash = hash64!(&string);
        assert_eq!(&string, &hash.to_string());
    }

    #[test]
    fn hash_macro_remembers_string_literal() {
        let string = "Foo-bar";
        let hash = hash64!(&string);
        assert_eq!(&string, &hash.to_string());
    }
}
