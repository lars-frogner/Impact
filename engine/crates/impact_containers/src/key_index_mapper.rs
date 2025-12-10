//! Map for keeping track of which [`HashMap`] key corresponds to which index in
//! an underlying [`Vec`].

use anyhow::{Result, anyhow};
use hashbrown::{HashMap, hash_map::Entry};
use impact_alloc::{AVec, Allocator, Global};
use rustc_hash::FxBuildHasher;
use std::fmt::{self, Debug};
use std::hash::{BuildHasher, Hash};
use std::iter;

/// Map for keeping track of which [`HashMap`] key corresponds to which index in
/// an underlying [`Vec`].
///
/// This is useful if we want the flexibility of accessing data with a key but
/// don't want to sacrifice the compact data storage provided by a `Vec`. It
/// also enables us to reorder items in the `Vec` (like doing a swap remove)
/// without invalidating the keys used to access the items.
pub struct KeyIndexMapper<K, S = FxBuildHasher, A: Allocator = Global> {
    indices_for_keys: HashMap<K, usize, S, A>,
    keys_at_indices: AVec<K, A>,
}

impl<K> KeyIndexMapper<K, FxBuildHasher, Global>
where
    K: Copy + Hash + Eq + Debug,
{
    /// Creates a new mapper with no keys.
    pub fn new() -> Self {
        Self::with_capacity_and_hasher(0, FxBuildHasher)
    }

    /// Creates a new mapper with at least the specificed capacity and no keys.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, FxBuildHasher)
    }

    /// Creates a new mapper with the given key.
    pub fn new_with_key(key: K) -> Self {
        Self::with_hasher_and_key(FxBuildHasher, key)
    }

    /// Creates a new mapper with the given set of keys. The index of each key
    /// will correspond to the position of the key in the provided iterator.
    ///
    /// # Panics
    /// If the iterator has multiple occurences of the same key.
    pub fn new_with_keys(key_iter: impl IntoIterator<Item = K>) -> Self {
        Self::with_hasher_and_keys(FxBuildHasher, key_iter)
    }
}

impl<K, A> KeyIndexMapper<K, FxBuildHasher, A>
where
    K: Copy + Hash + Eq + Debug,
    A: Allocator,
{
    /// Creates a new mapper with no keys. It will be allocated with the given
    /// allocator.
    pub fn new_in(alloc: A) -> Self {
        Self::with_capacity_and_hasher_in(0, FxBuildHasher, alloc)
    }

    /// Creates a new mapper with at least the specificed capacity and no keys.
    /// It will be allocated with the given allocator.
    pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
        Self::with_capacity_and_hasher_in(capacity, FxBuildHasher, alloc)
    }

    /// Creates a new mapper with the given key. It will be allocated with the
    /// given allocator.
    pub fn new_with_key_in(alloc: A, key: K) -> Self {
        Self::with_hasher_and_key_in(FxBuildHasher, alloc, key)
    }

    /// Creates a new mapper with the given set of keys. It will be allocated
    /// with the given allocator. The index of each key will correspond to the
    /// position of the key in the provided iterator.
    ///
    /// # Panics
    /// If the iterator has multiple occurences of the same key.
    pub fn new_with_keys_in(alloc: A, key_iter: impl IntoIterator<Item = K>) -> Self {
        Self::with_hasher_and_keys_in(FxBuildHasher, alloc, key_iter)
    }
}

impl<K, S> KeyIndexMapper<K, S, Global>
where
    K: Copy + Hash + Eq + Debug,
    S: BuildHasher + Default,
{
    /// Creates a new mapper with at least the specificed capacity, the given
    /// hasher and with no keys.
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self::with_capacity_and_hasher_in(capacity, hash_builder, Global)
    }

    /// Creates a new mapper with the given hasher and the given key.
    pub fn with_hasher_and_key(hash_builder: S, key: K) -> Self {
        Self::with_hasher_and_key_in(hash_builder, Global, key)
    }

    /// Creates a new mapper with the given hasher and the given set of keys.
    /// The index of each key will correspond to the position of the key in the
    /// provided iterator.
    ///
    /// # Panics
    /// If the iterator has multiple occurences of the same key.
    pub fn with_hasher_and_keys(hash_builder: S, key_iter: impl IntoIterator<Item = K>) -> Self {
        Self::with_hasher_and_keys_in(hash_builder, Global, key_iter)
    }
}

impl<K, S, A> KeyIndexMapper<K, S, A>
where
    K: Copy + Hash + Eq + Debug,
    S: BuildHasher + Default,
    A: Allocator,
{
    /// Creates a new mapper with at least the specificed capacity, the given
    /// hasher and with no keys. It will be allocated with the given allocator.
    pub fn with_capacity_and_hasher_in(capacity: usize, hash_builder: S, alloc: A) -> Self {
        Self {
            indices_for_keys: HashMap::with_capacity_and_hasher_in(capacity, hash_builder, alloc),
            keys_at_indices: AVec::with_capacity_in(capacity, alloc),
        }
    }

    /// Creates a new mapper with the given hasher and the given key. It will be
    /// allocated with the given allocator.
    pub fn with_hasher_and_key_in(hash_builder: S, alloc: A, key: K) -> Self {
        Self::with_hasher_and_keys_in(hash_builder, alloc, iter::once(key))
    }

    /// Creates a new mapper with the given hasher and the given set of keys. It
    /// will be allocated with the given allocator. The index of each key will
    /// correspond to the position of the key in the provided iterator.
    ///
    /// # Panics
    /// If the iterator has multiple occurences of the same key.
    pub fn with_hasher_and_keys_in(
        hash_builder: S,
        alloc: A,
        key_iter: impl IntoIterator<Item = K>,
    ) -> Self {
        let key_iter = key_iter.into_iter();
        let capacity = key_iter.size_hint().0;
        let mut mapper = Self::with_capacity_and_hasher_in(capacity, hash_builder, alloc);
        for key in key_iter {
            mapper.push_key(key);
        }
        mapper
    }

    /// Reserves capacity for at least `additional` more elements to be
    /// inserted.
    pub fn reserve(&mut self, additional: usize) {
        self.indices_for_keys.reserve(additional);
        self.keys_at_indices.reserve(additional);
    }

    /// Returns a reference to the [`HashMap`] of keys to indices.
    pub fn as_map(&self) -> &HashMap<K, usize, S, A> {
        &self.indices_for_keys
    }

    /// Consumes the mapper and returns the [`HashMap`] of keys to indices.
    pub fn into_map(self) -> HashMap<K, usize, S, A> {
        self.indices_for_keys
    }

    /// Returns an iterator over all keys in the order in which their entries in
    /// the underlying [`Vec`] are stored.
    pub fn key_at_each_idx(&self) -> impl Iterator<Item = K> {
        self.keys_at_indices.iter().copied()
    }

    /// Returns a slice with all keys in the order in which their entries in the
    /// underlying [`Vec`] are stored.
    pub fn keys_at_indices(&self) -> &[K] {
        &self.keys_at_indices
    }

    /// Whether the mapper has no keys.
    pub fn is_empty(&self) -> bool {
        self.keys_at_indices.is_empty()
    }

    /// Whether an index exists for the given key.
    pub fn contains_key(&self, key: K) -> bool {
        self.indices_for_keys.contains_key(&key)
    }

    /// The number of keys/indices in the mapper.
    pub fn len(&self) -> usize {
        self.keys_at_indices.len()
    }

    /// Returns the index corresponding to the given key.
    ///
    /// # Panics
    /// If the key does not exist.
    pub fn idx(&self, key: K) -> usize {
        self.indices_for_keys[&key]
    }

    /// Returns the index corresponding to the given key if the key exists,
    /// otherwise returns [`None`].
    pub fn get(&self, key: K) -> Option<usize> {
        self.indices_for_keys.get(&key).cloned()
    }

    /// Returns the key corresponding to the given index.
    ///
    /// # Panics
    /// If the index is outside the bounds of the [`Vec`].
    pub fn key_at_idx(&self, idx: usize) -> K {
        self.keys_at_indices[idx]
    }

    /// Adds the given key and maps it to the next index.
    ///
    /// # Errors
    /// Returns an error with the index of the key if the key already exists.
    pub fn try_push_key(&mut self, key: K) -> Result<(), usize> {
        match self.indices_for_keys.entry(key) {
            Entry::Vacant(entry) => {
                let idx_of_new_key = self.keys_at_indices.len();
                entry.insert(idx_of_new_key);
                self.keys_at_indices.push(key);
                Ok(())
            }
            Entry::Occupied(entry) => Err(*entry.get()),
        }
    }

    /// Adds the given key and maps it to the next index.
    ///
    /// # Panics
    /// If the key already exists.
    pub fn push_key(&mut self, key: K) {
        self.try_push_key(key)
            .expect("Tried to add an existing key");
    }

    /// Pushes each of the keys in the given iterator into the map in order.
    ///
    /// # Panics
    /// If any of the keys already exists.
    pub fn push_keys(&mut self, keys: impl IntoIterator<Item = K>) {
        keys.into_iter().for_each(|key| self.push_key(key));
    }

    /// Removes the given key and assigns the key at the last index to the index
    /// of the removed key (unless the key to remove was at the last index)
    /// before popping the end of the [`Vec`].
    ///
    /// # Returns
    /// The index of the removed key.
    ///
    /// # Errors
    /// Returns an error if the key to remove does not exist.
    pub fn try_swap_remove_key(&mut self, key: K) -> Result<usize> {
        let idx_of_removed_key = self
            .indices_for_keys
            .remove(&key)
            .ok_or_else(|| anyhow!("Tried to remove key that does not exist"))?;

        let last_key = self.keys_at_indices.pop().unwrap();
        if key != last_key {
            self.keys_at_indices[idx_of_removed_key] = last_key;
            *self.indices_for_keys.get_mut(&last_key).unwrap() = idx_of_removed_key;
        }
        Ok(idx_of_removed_key)
    }

    /// Removes the given key and assigns the key at the last index to the index
    /// of the removed key (unless the key to remove was at the last index)
    /// before popping the end of the [`Vec`].
    ///
    /// # Returns
    /// The index of the removed key.
    ///
    /// # Panics
    /// If the key to remove does not exist.
    pub fn swap_remove_key(&mut self, key: K) -> usize {
        self.try_swap_remove_key(key)
            .expect("Tried to remove key that does not exist")
    }

    /// Removes the key corresponding to the given index and assigns the key at
    /// the last index to the index of the removed key (unless the key to remove
    /// was at the last index) before popping the end of the [`Vec`].
    ///
    /// # Panics
    /// If the index is outside the bounds of the [`Vec`].
    pub fn swap_remove_key_at_idx(&mut self, idx: usize) {
        let last_key = *self.keys_at_indices.last().unwrap();
        let removed_key = self.keys_at_indices.swap_remove(idx);
        self.indices_for_keys.remove(&removed_key).unwrap();
        if removed_key != last_key {
            *self.indices_for_keys.get_mut(&last_key).unwrap() = idx;
        }
    }

    /// Clears all stored indices and keys.
    pub fn clear(&mut self) {
        self.indices_for_keys.clear();
        self.keys_at_indices.clear();
    }
}

impl<K, S, A> Clone for KeyIndexMapper<K, S, A>
where
    K: Clone,
    S: Clone,
    A: Allocator + Clone,
{
    fn clone(&self) -> Self {
        Self {
            indices_for_keys: self.indices_for_keys.clone(),
            keys_at_indices: self.keys_at_indices.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.indices_for_keys.clone_from(&source.indices_for_keys);
        self.keys_at_indices.clone_from(&source.keys_at_indices);
    }
}

impl<K, S, A> Default for KeyIndexMapper<K, S, A>
where
    K: Copy + Hash + Eq + Debug,
    S: BuildHasher + Default,
    A: Allocator + Default,
{
    fn default() -> Self {
        Self::with_capacity_and_hasher_in(0, S::default(), A::default())
    }
}

impl<K: fmt::Debug, S> fmt::Debug for KeyIndexMapper<K, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyIndexMapper")
            .field("indices_for_keys", &self.indices_for_keys)
            .field("keys_at_indices", &self.keys_at_indices)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_index_mapper_initialization_works() {
        let mapper = KeyIndexMapper::<i32>::new();
        assert!(mapper.is_empty());

        let mapper = KeyIndexMapper::new_with_key(3);
        assert_eq!(mapper.len(), 1);
        assert_eq!(mapper.idx(3), 0);
        assert_eq!(mapper.key_at_idx(0), 3);

        let mapper = KeyIndexMapper::new_with_keys([4, 2]);
        assert_eq!(mapper.len(), 2);
        assert_eq!(mapper.idx(4), 0);
        assert_eq!(mapper.idx(2), 1);
        assert_eq!(mapper.key_at_idx(0), 4);
        assert_eq!(mapper.key_at_idx(1), 2);
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_initializing_with_duplicate_keys_fails() {
        KeyIndexMapper::new_with_keys([2, 4, 2]);
    }

    #[test]
    fn key_index_mapper_key_at_each_idx_gives_correct_keys() {
        let mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        let mut iter = mapper.key_at_each_idx();
        assert_eq!(iter.next().unwrap(), 4);
        assert_eq!(iter.next().unwrap(), 2);
        assert_eq!(iter.next().unwrap(), 100);
        assert!(iter.next().is_none());
    }

    #[test]
    fn key_index_mapper_keys_at_indices_gives_correct_keys() {
        let mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        assert_eq!(mapper.keys_at_indices(), &[4, 2, 100]);
    }

    #[test]
    fn key_index_mapper_get_gives_correct_idx() {
        let mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        assert_eq!(mapper.get(0), None);
        assert_eq!(mapper.get(4), Some(0));
        assert_eq!(mapper.get(2), Some(1));
        assert_eq!(mapper.get(100), Some(2));
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_idx_fails_on_invalid_key() {
        let mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        mapper.idx(0);
    }

    #[test]
    fn key_index_mapper_idx_gives_correct_idx() {
        let mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        assert_eq!(mapper.idx(4), 0);
        assert_eq!(mapper.idx(2), 1);
        assert_eq!(mapper.idx(100), 2);
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_key_at_idx_fails_on_invalid_idx() {
        let mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        mapper.key_at_idx(3);
    }

    #[test]
    fn key_index_mapper_key_at_idx_gives_correct_key() {
        let mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        assert_eq!(mapper.key_at_idx(0), 4);
        assert_eq!(mapper.key_at_idx(1), 2);
        assert_eq!(mapper.key_at_idx(2), 100);
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_pushing_duplicate_keys_fails() {
        let mut mapper = KeyIndexMapper::<i32>::new();
        mapper.push_key(4);
        mapper.push_key(4);
    }

    #[test]
    fn key_index_mapper_try_push_key_err_contains_existing_idx() {
        let mut mapper = KeyIndexMapper::<i32>::new();
        mapper.push_key(4);
        assert_eq!(mapper.try_push_key(4).unwrap_err(), 0);

        mapper.push_key(7);
        assert_eq!(mapper.try_push_key(7).unwrap_err(), 1);
        assert_eq!(mapper.try_push_key(4).unwrap_err(), 0);
    }

    #[test]
    fn key_index_mapper_push_key_works() {
        let mut mapper = KeyIndexMapper::<i32>::new();
        assert!(mapper.is_empty());

        mapper.push_key(4);
        assert_eq!(mapper.idx(4), 0);
        assert_eq!(mapper.key_at_idx(0), 4);

        mapper.push_key(100);
        assert_eq!(mapper.idx(4), 0);
        assert_eq!(mapper.key_at_idx(0), 4);
        assert_eq!(mapper.idx(100), 1);
        assert_eq!(mapper.key_at_idx(1), 100);
    }

    #[test]
    fn key_index_mapper_pushing_multiple_keys_works() {
        let mut mapper = KeyIndexMapper::<i32>::new();

        mapper.push_keys([4, 100]);
        assert_eq!(mapper.idx(4), 0);
        assert_eq!(mapper.key_at_idx(0), 4);
        assert_eq!(mapper.idx(100), 1);
        assert_eq!(mapper.key_at_idx(1), 100);
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_swap_remove_idx_on_empty_fails() {
        let mut mapper = KeyIndexMapper::<i32>::new();
        mapper.swap_remove_key_at_idx(0);
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_swap_remove_idx_with_invalid_idx_fails() {
        let mut mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        mapper.swap_remove_key_at_idx(3);
    }

    #[test]
    fn key_index_mapper_swap_remove_idx_works() {
        let mut mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);

        mapper.swap_remove_key_at_idx(0); // Moves `100` to idx 0 and truncates ([100, 2])
        assert_eq!(mapper.len(), 2);
        assert_eq!(mapper.idx(100), 0);
        assert_eq!(mapper.key_at_idx(0), 100);
        assert_eq!(mapper.idx(2), 1);
        assert_eq!(mapper.key_at_idx(1), 2);

        mapper.swap_remove_key_at_idx(1); // Truncates `2` ([100])
        assert_eq!(mapper.len(), 1);
        assert_eq!(mapper.idx(100), 0);
        assert_eq!(mapper.key_at_idx(0), 100);

        mapper.swap_remove_key_at_idx(0);
        assert!(mapper.is_empty());
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_swap_remove_key_on_empty_fails() {
        let mut mapper = KeyIndexMapper::<i32>::new();
        mapper.swap_remove_key(0);
    }

    #[test]
    #[should_panic]
    fn key_index_mapper_swap_remove_key_with_invalid_key_fails() {
        let mut mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);
        mapper.swap_remove_key(1);
    }

    #[test]
    fn key_index_mapper_swap_remove_key_works() {
        let mut mapper = KeyIndexMapper::new_with_keys([4, 2, 100]);

        mapper.swap_remove_key(2); // Moves `100` to idx 1 and truncates ([4, 100])
        assert_eq!(mapper.len(), 2);
        assert_eq!(mapper.idx(4), 0);
        assert_eq!(mapper.key_at_idx(0), 4);
        assert_eq!(mapper.idx(100), 1);
        assert_eq!(mapper.key_at_idx(1), 100);

        mapper.swap_remove_key(100); // Truncates `100` ([4])
        assert_eq!(mapper.len(), 1);
        assert_eq!(mapper.idx(4), 0);
        assert_eq!(mapper.key_at_idx(0), 4);

        mapper.swap_remove_key(4);
        assert!(mapper.is_empty());
    }
}
