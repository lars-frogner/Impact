//! Map for keeping track of which [`HashMap`] key
//! corresponds to which index in an underlying [`Vec`].

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Map for keeping track of which [`HashMap`] key
/// corresponds to which index in an underlying [`Vec`].
///
/// This is useful if we want the flexibility of
/// accessing data with a key but don't want to
/// sacrifice the compact data storage provided by a
/// `Vec`. It also enables us to reorder items in
/// the `Vec` (like doing a swap remove) without
/// invalidating the keys used to access the items.
#[derive(Clone, Debug)]
pub struct KeyIndexMapper<K> {
    indices_for_keys: HashMap<K, usize>,
    keys_at_indices: Vec<K>,
}

impl<K> KeyIndexMapper<K>
where
    K: Copy + Hash + Eq + Debug,
{
    /// Creates a new mapper with no keys.
    pub fn new() -> Self {
        Self {
            indices_for_keys: HashMap::new(),
            keys_at_indices: Vec::new(),
        }
    }

    /// Creates a new mapper with the given key.
    #[allow(dead_code)]
    pub fn new_with_key(key: K) -> Self {
        Self {
            indices_for_keys: [(key, 0)].into_iter().collect(),
            keys_at_indices: vec![key],
        }
    }

    /// Creates a new mapper with the given set of keys.
    /// The index of each key will correspond to the position
    /// of the key in the provided iterator.
    ///
    /// # Panics
    /// If the iterator has multiple occurences of the same key.
    pub fn new_with_keys(key_iter: impl IntoIterator<Item = K>) -> Self {
        let mut mapper = Self::new();
        for key in key_iter {
            mapper.push_key(key);
        }
        mapper
    }

    /// Consumes the mapper and returns the [`HashMap`] of
    /// keys to indices.
    pub fn into_map(self) -> HashMap<K, usize> {
        self.indices_for_keys
    }

    /// Returns an iterator over all keys in the order in
    /// which their entries in the underlying [`Vec`] are
    /// stored.
    pub fn key_at_each_idx(&self) -> impl Iterator<Item = K> + '_ {
        self.keys_at_indices.iter().copied()
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

    /// Returns the index corresponding to the given key
    /// if the key exists, otherwise returns [`None`].
    pub fn get(&self, key: K) -> Option<usize> {
        self.indices_for_keys.get(&key).cloned()
    }

    /// Returns the key corresponding to the given index.
    ///
    /// # Panics
    /// If the index is outside the bounds of the [`Vec`].
    #[allow(dead_code)]
    pub fn key_at_idx(&self, idx: usize) -> K {
        self.keys_at_indices[idx]
    }

    /// Adds the given key and maps it to the next index.
    ///
    /// # Panics
    /// If the key already exists.
    pub fn push_key(&mut self, key: K) {
        let idx_of_new_key = self.keys_at_indices.len();
        self.keys_at_indices.push(key);
        let existing_entry = self.indices_for_keys.insert(key, idx_of_new_key);
        assert!(existing_entry.is_none(), "Tried to add an existing key");
    }

    /// Pushes each of the keys in the given iterator into the map
    /// in order.
    ///
    /// # Panics
    /// If any of the keys already exists.
    pub fn push_keys(&mut self, keys: impl IntoIterator<Item = K>) {
        keys.into_iter().for_each(|key| self.push_key(key));
    }

    /// Removes the given key and assigns the key at the last
    /// index to the index of the removed key (unless the key
    /// to remove was at the last index) before popping the end
    /// of the [`Vec`].
    ///
    /// # Returns
    /// The index of the removed key.
    ///
    /// # Panics
    /// If the key to remove does not exist.
    pub fn swap_remove_key(&mut self, key: K) -> usize {
        let idx_of_removed_key = self.indices_for_keys.remove(&key).unwrap();
        let last_key = self.keys_at_indices.pop().unwrap();
        if key != last_key {
            self.keys_at_indices[idx_of_removed_key] = last_key;
            *self.indices_for_keys.get_mut(&last_key).unwrap() = idx_of_removed_key;
        }
        idx_of_removed_key
    }

    /// Removes the key corresponding to the given index and
    /// assigns the key at the last index to the index of the
    /// removed key (unless the key to remove was at the last
    /// index) before popping the end of the [`Vec`].
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
}

impl<K> Default for KeyIndexMapper<K>
where
    K: Copy + Hash + Eq + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
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
