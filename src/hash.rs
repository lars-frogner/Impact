use lazy_static::lazy_static;
use std::{
    cmp,
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    sync::Mutex,
};

/// A 64-bit hash of a string.
///
/// This object remembers the original string and can be
/// formatted into it by means of the [`Display`](fmt::Display)
/// trait.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StringHash {
    string_hash: u64,
    string: Option<&'static str>,
}

lazy_static! {
    static ref STRING_HASH_REGISTRY: Mutex<HashMap<u64, String>> = Mutex::new(HashMap::new());
}

impl StringHash {
    /// Creates a hash of the given string literal. This method
    /// is evaluated at compile time.
    ///
    /// The reference to the string literal is stored together
    /// with the hash so that it can be retrieved.
    pub const fn of_literal(string: &'static str) -> Self {
        let string_hash = Self::compute_hash(string);
        Self {
            string_hash,
            string: Some(string),
        }
    }

    /// Creates a hash of the given [`String`].
    ///
    /// The string and associated hash are inserted into a global
    /// registry so that the string can be looked up.
    ///
    /// # Concurrency
    /// The method has to temorarily acquire a lock on the global
    /// string registry in order to record the hash and string pair.
    pub fn of_owned(string: String) -> Self {
        let string_hash = Self::compute_hash(string.as_str());

        STRING_HASH_REGISTRY
            .lock()
            .unwrap()
            .entry(string_hash)
            .or_insert(string);

        Self {
            string_hash,
            string: None,
        }
    }

    const fn compute_hash(string: &str) -> u64 {
        const_fnv1a_hash::fnv1a_hash_str_64(string)
    }
}

impl fmt::Display for StringHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.string {
            Some(string) => {
                // Use stored string literal
                write!(f, "{}", string)
            }
            None => {
                // Look up string in registry
                write!(
                    f,
                    "{}",
                    STRING_HASH_REGISTRY
                        .lock()
                        .unwrap()
                        .get(&self.string_hash)
                        .expect("Missing entry for hash in global string hash registry")
                )
            }
        }
    }
}

impl Ord for StringHash {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.string_hash.cmp(&other.string_hash)
    }
}

impl PartialOrd for StringHash {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Disabling this error because the requirement for `Hash`,
// `k1 == k2 -> hash(k1) == hash(k2)`, is still upheld
// even though we only hash one of the fields
#[allow(clippy::derive_hash_xor_eq)]
impl Hash for StringHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.string_hash.hash(state);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hash_remembers_string_literal() {
        let string = "Foo-bar";
        let hash = StringHash::of_literal(string);
        assert_eq!(string, &hash.to_string());
    }

    #[test]
    fn hash_remembers_owned_string() {
        let string = "Foo-bar".to_string();
        let hash = StringHash::of_owned(string.clone());
        assert_eq!(string, hash.to_string());
    }
}
