//! A [`Vec`] that maintains a list of each index where the element has been
//! deleted and reuses these locations when adding new items.

use bytemuck::{Pod, Zeroable};
use roc_integration::roc;
use std::cmp;

/// A [`Vec`] that maintains a list of each index where the element has been
/// removed and reuses these locations when adding new items.
///
/// In order to prevent use-after-free issues, each location ("slot") has an
/// associated "generation" that is advanced every time the slot is reused after
/// a removal. The generation is contained in the returned [`SlotKey`] when a
/// value is inserted. Every time a value is to be accessed, the generation of
/// the key is compared to the current generation of the slot, and the access is
/// rejected if the generations do not match.
#[derive(Clone, Debug, Default)]
pub struct SlotMap<V> {
    slots: Vec<Slot<V>>,
    free_slot_keys: Vec<SlotKey>,
}

/// A key into a [`SlotMap`].
#[roc(parents = "Containers")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
pub struct SlotKey {
    generation: Generation,
    idx: u32,
}

#[derive(Clone, Debug)]
struct Slot<V> {
    generation: Generation,
    value: V,
}

#[roc(parents = "Containers")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
struct Generation(u32);

impl<V> SlotMap<V> {
    /// Creates a new empty map.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_slot_keys: Vec::new(),
        }
    }

    /// Returns the number of values in the map.
    ///
    /// The actual number of allocated slots may be higher than this.
    pub fn len(&self) -> usize {
        self.slots.len() - self.free_slot_keys.len()
    }

    /// Returns true if the map contains no values.
    ///
    /// The map may still have allocated slots.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a reference to the value for the given key.
    ///
    /// # Panics
    /// If the key:
    /// - Refers to a slot that is currently free or has been reused.
    /// - Has a generation representing a free slot (in which case it is illegal
    ///   as a key).
    /// - Has an out of bounds index (in which case it belongs to a different
    ///   map).
    pub fn value(&self, key: SlotKey) -> &V {
        self.get_value(key)
            .expect("Tried to access free or reused slot")
    }

    /// Returns a mutable reference to the value for the given key.
    ///
    /// # Panics
    /// If the key:
    /// - Refers to a slot that is currently free or has been reused.
    /// - Has a generation representing a free slot (in which case it is illegal
    ///   as a key).
    /// - Has an out of bounds index (in which case it belongs to a different
    ///   map).
    pub fn value_mut(&mut self, key: SlotKey) -> &mut V {
        self.get_value_mut(key)
            .expect("Tried to access free or reused slot")
    }

    /// Returns a reference to the value for the given key, or [`None`] if the
    /// value has been removed.
    ///
    /// # Panics
    /// If the key's generation represents a free slot (in which case it is
    /// illegal as a key) or its index is out of bounds (in which case it
    /// belongs to a different map).
    pub fn get_value(&self, key: SlotKey) -> Option<&V> {
        assert!(key.is_legal(), "Tried to use illegal slot key");
        self.slots[key.idx_usize()].get_value(key.generation())
    }

    /// Returns a mutable reference to the value for the given key, or [`None`]
    /// if the value has been removed.
    ///
    /// # Panics
    /// If the key's generation represents a free slot (in which case it is
    /// illegal as a key) or its index is out of bounds (in which case it
    /// belongs to a different map).
    pub fn get_value_mut(&mut self, key: SlotKey) -> Option<&mut V> {
        assert!(key.is_legal(), "Tried to use illegal slot key");
        self.slots[key.idx_usize()].get_value_mut(key.generation())
    }

    /// Whether a value exists for the given key.
    ///
    /// # Panics
    /// If the key's generation represents a free slot (in which case it is
    /// illegal as a key) or its index is out of bounds (in which case it
    /// belongs to a different map).
    pub fn contains(&self, key: SlotKey) -> bool {
        assert!(key.is_legal(), "Tried to use illegal slot key");
        self.slots[key.idx_usize()].has_generation(key.generation())
    }

    /// Returns an iterator over the keys and values in the map.
    pub fn iter(&self) -> impl Iterator<Item = (SlotKey, &V)> {
        self.slots.iter().enumerate().filter_map(|(idx, slot)| {
            if slot.is_free() {
                None
            } else {
                Some((SlotKey::new(slot.generation(), idx as u32), &slot.value))
            }
        })
    }

    /// Returns an iterator over the keys and values in the map that allows
    /// modifying each value.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (SlotKey, &mut V)> {
        self.slots.iter_mut().enumerate().filter_map(|(idx, slot)| {
            if slot.is_free() {
                None
            } else {
                Some((SlotKey::new(slot.generation(), idx as u32), &mut slot.value))
            }
        })
    }

    /// Returns an iterator over the keys in the map.
    pub fn keys(&self) -> impl Iterator<Item = SlotKey> {
        self.slots.iter().enumerate().filter_map(|(idx, slot)| {
            if slot.is_free() {
                None
            } else {
                Some(SlotKey::new(slot.generation(), idx as u32))
            }
        })
    }

    /// Returns an iterator over the values in the map.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.slots.iter().filter_map(|slot| {
            if slot.is_free() {
                None
            } else {
                Some(&slot.value)
            }
        })
    }

    /// Returns an iterator over the values in the map that allows modifying
    /// each value.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.slots.iter_mut().filter_map(|slot| {
            if slot.is_free() {
                None
            } else {
                Some(&mut slot.value)
            }
        })
    }

    /// Inserts the given value into the map. If a free slot is available, this
    /// is used, otherwise the value is inserted into a new slot at the end.
    ///
    /// If the value is inserted into a free slot, the generation of this slot
    /// is advanced. This makes it impossible for (now invalidated) previous
    /// keys referring to the same slot to access the new value.
    ///
    /// # Returns
    /// The key for the slot where the value was added.
    pub fn insert(&mut self, value: V) -> SlotKey {
        if let Some(free_slot_key) = self.free_slot_keys.pop() {
            let slot = &mut self.slots[free_slot_key.idx_usize()];
            let next_generation = free_slot_key.generation().next();

            slot.set_value_and_generation(value, next_generation);

            SlotKey::new(next_generation, free_slot_key.idx())
        } else {
            let key = SlotKey::new_first_generation(
                self.slots
                    .len()
                    .try_into()
                    .expect("Slot map exceeded maximum capacity (u32::MAX)"),
            );
            self.slots.push(Slot::new_first_generation(value));
            key
        }
    }

    /// Removes the value for the given key. The underlying [`Vec`] is not
    /// modified, instead the slot is registered as free.
    ///
    /// # Returns
    /// `true` if the value existed.
    ///
    /// # Panics
    /// If the key's generation represents a free slot (in which case it is
    /// illegal as a key) or its index is out of bounds (in which case it
    /// belongs to a different map).
    pub fn remove(&mut self, key: SlotKey) -> bool {
        assert!(key.is_legal(), "Tried to use illegal slot key");
        let slot = &mut self.slots[key.idx_usize()];
        if key.generation() != slot.generation() {
            // Already removed
            return false;
        }
        self.free_slot_keys.push(key);
        slot.declare_free();
        true
    }

    /// Remove all values by marking every occupied slot as free.
    pub fn clear(&mut self) {
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if !slot.is_free() {
                let key = SlotKey::new(slot.generation(), idx as u32);
                self.free_slot_keys.push(key);
                slot.declare_free();
            }
        }
    }
}

impl SlotKey {
    /// Sometimes we need to create a key without inserting a value into a
    /// `SlotMap`. This creates an illegal dummy key that will never be returned
    /// from a `SlotMap`, and produce a panic if actually used with a `SlotMap`.
    pub fn dummy() -> Self {
        Self {
            generation: Generation::free(),
            idx: 0,
        }
    }

    fn new(generation: Generation, idx: u32) -> Self {
        assert!(!generation.is_free());
        Self { generation, idx }
    }

    fn new_first_generation(idx: u32) -> Self {
        Self::new(Generation::first(), idx)
    }

    fn idx(&self) -> u32 {
        self.idx
    }

    fn idx_usize(&self) -> usize {
        self.idx as usize
    }

    fn generation(&self) -> Generation {
        self.generation
    }

    /// Illegal keys can be created through [`Self::dummy`] or
    /// [`Zeroable::zeroed`]. They will never be returned by a `SlotMap`.
    fn is_legal(&self) -> bool {
        !self.generation.is_free()
    }
}

impl Ord for SlotKey {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}

impl PartialOrd for SlotKey {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<V> Slot<V> {
    fn new_first_generation(value: V) -> Self {
        Self {
            generation: Generation::first(),
            value,
        }
    }

    fn generation(&self) -> Generation {
        self.generation
    }

    fn is_free(&self) -> bool {
        self.generation.is_free()
    }

    fn has_generation(&self, generation: Generation) -> bool {
        generation == self.generation
    }

    fn get_value(&self, generation: Generation) -> Option<&V> {
        if self.has_generation(generation) {
            Some(&self.value)
        } else {
            None
        }
    }

    fn get_value_mut(&mut self, generation: Generation) -> Option<&mut V> {
        if self.has_generation(generation) {
            Some(&mut self.value)
        } else {
            None
        }
    }

    fn set_value_and_generation(&mut self, value: V, new_generation: Generation) {
        self.value = value;
        self.generation = new_generation;
    }

    fn declare_free(&mut self) {
        self.generation.declare_free();
    }
}

impl Generation {
    fn first() -> Self {
        Self(1)
    }

    fn free() -> Self {
        Self(0)
    }

    fn is_free(&self) -> bool {
        self.0 == 0
    }

    fn declare_free(&mut self) {
        self.0 = 0;
    }

    fn next(&self) -> Self {
        assert!(!self.is_free());
        if self.0 < u32::MAX {
            Self(self.0 + 1)
        } else {
            Self::first() // Wrap on overflow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creating_map_works() {
        let map = SlotMap::<f32>::new();
        assert_eq!(map.len(), 0);
    }

    #[test]
    #[should_panic]
    fn demanding_value_from_empty_map_fails() {
        let map = SlotMap::<f32>::new();
        map.value(SlotKey::new_first_generation(0));
    }

    #[test]
    #[should_panic]
    fn demanding_mutable_value_from_empty_map_fails() {
        let mut map = SlotMap::<f32>::new();
        map.value_mut(SlotKey::new_first_generation(0));
    }

    #[test]
    #[should_panic]
    fn requesting_value_from_empty_map_fails() {
        let map = SlotMap::<f32>::new();
        map.get_value(SlotKey::new_first_generation(0));
    }

    #[test]
    #[should_panic]
    fn requesting_mutable_value_from_empty_map_fails() {
        let mut map = SlotMap::<f32>::new();
        map.get_value_mut(SlotKey::new_first_generation(0));
    }

    #[test]
    fn adding_to_empty_map_works() {
        let mut map = SlotMap::<f32>::new();
        let key = map.insert(1.0);

        assert_eq!(map.len(), 1);

        assert_eq!(*map.value(key), 1.0);
        assert_eq!(*map.value_mut(key), 1.0);

        assert_eq!(map.get_value(key), Some(&1.0));
        assert_eq!(map.get_value_mut(key), Some(&mut 1.0));
    }

    #[test]
    #[should_panic]
    fn removing_with_out_of_bounds_key_fails() {
        let mut map = SlotMap::<f32>::new();
        map.remove(SlotKey::new_first_generation(0));
    }

    #[test]
    fn removing_only_value_in_map_works() {
        let mut map = SlotMap::<f32>::new();
        let key = map.insert(1.0);
        assert!(map.remove(key));

        assert_eq!(map.len(), 0);
        assert_eq!(map.get_value(key), None);
        assert_eq!(map.get_value_mut(key), None);
    }

    #[test]
    #[should_panic]
    fn demanding_removed_value_fails() {
        let mut map = SlotMap::<f32>::new();
        let key = map.insert(1.0);
        map.remove(key);
        map.value(key);
    }

    #[test]
    #[should_panic]
    fn demanding_removed_mutable_value_fails() {
        let mut map = SlotMap::<f32>::new();
        let key = map.insert(1.0);
        map.remove(key);
        map.value_mut(key);
    }

    #[test]
    fn requesting_removed_value_gives_none() {
        let mut map = SlotMap::<f32>::new();
        let key = map.insert(1.0);
        assert!(map.remove(key));
        assert!(map.get_value(key).is_none());
    }

    #[test]
    fn requesting_removed_mutable_value_gives_none() {
        let mut map = SlotMap::<f32>::new();
        let key = map.insert(1.0);
        assert!(map.remove(key));
        assert!(map.get_value_mut(key).is_none());
    }

    #[test]
    fn removing_removed_value_does_nothing() {
        let mut map = SlotMap::<f32>::new();
        let key = map.insert(1.0);
        assert!(map.remove(key));
        assert!(!map.remove(key));
    }

    #[test]
    fn adding_after_remove_uses_free_slot() {
        let mut map = SlotMap::<f32>::new();
        let first_key = map.insert(0.0);
        assert_eq!(first_key.idx(), 0);
        assert_eq!(map.len(), 1);
        assert_eq!(*map.value(first_key), 0.0);

        let second_key = map.insert(1.0);
        assert_eq!(second_key.idx(), 1);
        assert_eq!(map.len(), 2);
        assert_eq!(*map.value(second_key), 1.0);

        map.remove(first_key);
        assert_eq!(map.len(), 1);

        let third_key = map.insert(2.0);
        assert_eq!(third_key.idx(), first_key.idx());
        assert_eq!(map.len(), 2);
        assert_eq!(*map.value(third_key), 2.0);

        let fourth_key = map.insert(3.0);
        assert_eq!(fourth_key.idx(), 2);
        assert_eq!(map.len(), 3);
        assert_eq!(*map.value(fourth_key), 3.0);
    }

    #[test]
    #[should_panic]
    fn demanding_value_with_outdated_generation_fails() {
        let mut map = SlotMap::<f32>::new();
        let first_key = map.insert(0.0);
        map.remove(first_key);
        map.insert(1.0);
        map.value(first_key);
    }

    #[test]
    #[should_panic]
    fn demanding_mutable_value_with_outdated_generation_fails() {
        let mut map = SlotMap::<f32>::new();
        let first_key = map.insert(0.0);
        map.remove(first_key);
        map.insert(1.0);
        map.value_mut(first_key);
    }

    #[test]
    fn requesting_value_with_outdated_generation_gives_none() {
        let mut map = SlotMap::<f32>::new();
        let first_key = map.insert(0.0);
        map.remove(first_key);
        map.insert(1.0);
        assert!(map.get_value(first_key).is_none());
    }

    #[test]
    fn requesting_mutable_value_with_outdated_generation_gives_none() {
        let mut map = SlotMap::<f32>::new();
        let first_key = map.insert(0.0);
        map.remove(first_key);
        map.insert(1.0);
        assert!(map.get_value_mut(first_key).is_none());
    }

    #[test]
    fn freeing_all_values_for_empty_map_works() {
        let mut map = SlotMap::<f32>::new();
        map.clear();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn freeing_all_values_for_single_value_map_works() {
        let mut map = SlotMap::<f32>::new();
        let first_key = map.insert(0.0);
        map.clear();
        assert_eq!(map.len(), 0);
        assert!(!map.contains(first_key));
    }

    #[test]
    fn freeing_all_values_for_multi_value_map_works() {
        let mut map = SlotMap::<f32>::new();
        let first_key = map.insert(0.0);
        let second_key = map.insert(1.0);
        map.clear();
        assert_eq!(map.len(), 0);
        assert!(!map.contains(first_key));
        assert!(!map.contains(second_key));
    }

    #[test]
    fn reusing_map_after_freeing_all_values_works() {
        let mut map = SlotMap::<f32>::new();
        let key_before_free = map.insert(0.0);
        map.clear();
        let key_after_free = map.insert(1.0);
        assert_eq!(map.len(), 1);
        assert!(!map.contains(key_before_free));
        assert_eq!(*map.value(key_after_free), 1.0);
    }

    #[test]
    fn multiple_removes_and_reuses_work() {
        let mut map = SlotMap::<i32>::new();

        // Insert several values
        let key1 = map.insert(10);
        let key2 = map.insert(20);
        let key3 = map.insert(30);
        let key4 = map.insert(40);
        assert_eq!(map.len(), 4);

        // Remove some values
        map.remove(key2);
        map.remove(key4);
        assert_eq!(map.len(), 2);

        // Insert new values, should reuse freed slots
        let key5 = map.insert(50);
        let key6 = map.insert(60);
        assert_eq!(map.len(), 4);

        // Check that old keys are invalid and new keys work
        assert!(!map.contains(key2));
        assert!(!map.contains(key4));
        assert_eq!(*map.value(key5), 50);
        assert_eq!(*map.value(key6), 60);

        // Original remaining keys should still work
        assert_eq!(*map.value(key1), 10);
        assert_eq!(*map.value(key3), 30);
    }

    #[test]
    fn clear_on_map_with_mixed_free_and_occupied_slots_works() {
        let mut map = SlotMap::<i32>::new();

        // Insert several values
        let key1 = map.insert(1);
        let key2 = map.insert(2);
        let key3 = map.insert(3);
        let key4 = map.insert(4);

        // Remove some values
        map.remove(key2);
        map.remove(key4);

        // Clear should handle both occupied and already-free slots
        map.clear();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());

        // All keys should be invalid
        assert!(!map.contains(key1));
        assert!(!map.contains(key2));
        assert!(!map.contains(key3));
        assert!(!map.contains(key4));
    }

    #[test]
    #[should_panic]
    fn using_dummy_key_fails() {
        let map = SlotMap::<f32>::new();
        let dummy_key = SlotKey::dummy();
        map.get_value(dummy_key);
    }

    #[test]
    #[should_panic]
    fn using_dummy_key_for_mutation_fails() {
        let mut map = SlotMap::<f32>::new();
        let dummy_key = SlotKey::dummy();
        map.get_value_mut(dummy_key);
    }

    #[test]
    #[should_panic]
    fn removing_with_dummy_key_fails() {
        let mut map = SlotMap::<f32>::new();
        let dummy_key = SlotKey::dummy();
        map.remove(dummy_key);
    }

    #[test]
    fn next_generation_after_max_is_not_free() {
        assert!(!Generation(u32::MAX).next().is_free());
    }

    #[test]
    fn iter_on_empty_map_returns_no_items() {
        let map = SlotMap::<i32>::new();
        assert_eq!(map.iter().count(), 0);
    }

    #[test]
    fn iter_on_single_item_map_works() {
        let mut map = SlotMap::<i32>::new();
        let key = map.insert(42);

        let items: Vec<_> = map.iter().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, key);
        assert_eq!(*items[0].1, 42);
    }

    #[test]
    fn iter_on_multiple_items_works() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);
        let key3 = map.insert(30);

        let mut items: Vec<_> = map.iter().collect();
        items.sort_by_key(|(k, _)| k.idx());

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].0, key1);
        assert_eq!(*items[0].1, 10);
        assert_eq!(items[1].0, key2);
        assert_eq!(*items[1].1, 20);
        assert_eq!(items[2].0, key3);
        assert_eq!(*items[2].1, 30);
    }

    #[test]
    fn iter_skips_removed_items() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);
        let key3 = map.insert(30);

        map.remove(key2);

        let mut items: Vec<_> = map.iter().collect();
        items.sort_by_key(|(k, _)| k.idx());

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, key1);
        assert_eq!(*items[0].1, 10);
        assert_eq!(items[1].0, key3);
        assert_eq!(*items[1].1, 30);
    }

    #[test]
    fn iter_mut_on_empty_map_returns_no_items() {
        let mut map = SlotMap::<i32>::new();
        assert_eq!(map.iter_mut().count(), 0);
    }

    #[test]
    fn iter_mut_allows_modification() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);

        for (_, value) in map.iter_mut() {
            *value += 5;
        }

        assert_eq!(*map.value(key1), 15);
        assert_eq!(*map.value(key2), 25);
    }

    #[test]
    fn iter_mut_skips_removed_items() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);
        let key3 = map.insert(30);

        map.remove(key2);

        let mut items: Vec<_> = map.iter_mut().collect();
        items.sort_by_key(|(k, _)| k.idx());

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, key1);
        assert_eq!(*items[0].1, 10);
        assert_eq!(items[1].0, key3);
        assert_eq!(*items[1].1, 30);
    }

    #[test]
    fn keys_on_empty_map_returns_no_keys() {
        let map = SlotMap::<i32>::new();
        assert_eq!(map.keys().count(), 0);
    }

    #[test]
    fn keys_returns_all_valid_keys() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);
        let key3 = map.insert(30);

        let mut keys: Vec<_> = map.keys().collect();
        keys.sort_by_key(|k| k.idx());

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], key1);
        assert_eq!(keys[1], key2);
        assert_eq!(keys[2], key3);
    }

    #[test]
    fn keys_skips_removed_items() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);
        let key3 = map.insert(30);

        map.remove(key2);

        let mut keys: Vec<_> = map.keys().collect();
        keys.sort_by_key(|k| k.idx());

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], key1);
        assert_eq!(keys[1], key3);
    }

    #[test]
    fn values_on_empty_map_returns_no_values() {
        let map = SlotMap::<i32>::new();
        assert_eq!(map.values().count(), 0);
    }

    #[test]
    fn values_returns_all_valid_values() {
        let mut map = SlotMap::<i32>::new();
        let _key1 = map.insert(10);
        let _key2 = map.insert(20);
        let _key3 = map.insert(30);

        let mut values: Vec<_> = map.values().copied().collect();
        values.sort();

        assert_eq!(values.len(), 3);
        assert_eq!(values, vec![10, 20, 30]);
    }

    #[test]
    fn values_skips_removed_items() {
        let mut map = SlotMap::<i32>::new();
        let _key1 = map.insert(10);
        let key2 = map.insert(20);
        let _key3 = map.insert(30);

        map.remove(key2);

        let mut values: Vec<_> = map.values().copied().collect();
        values.sort();

        assert_eq!(values.len(), 2);
        assert_eq!(values, vec![10, 30]);
    }

    #[test]
    fn values_mut_on_empty_map_returns_no_values() {
        let mut map = SlotMap::<i32>::new();
        assert_eq!(map.values_mut().count(), 0);
    }

    #[test]
    fn values_mut_allows_modification() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);
        let key3 = map.insert(30);

        for value in map.values_mut() {
            *value *= 2;
        }

        assert_eq!(*map.value(key1), 20);
        assert_eq!(*map.value(key2), 40);
        assert_eq!(*map.value(key3), 60);
    }

    #[test]
    fn values_mut_skips_removed_items() {
        let mut map = SlotMap::<i32>::new();
        let _key1 = map.insert(10);
        let key2 = map.insert(20);
        let _key3 = map.insert(30);

        map.remove(key2);

        let mut values: Vec<_> = map.values_mut().map(|v| *v).collect();
        values.sort();

        assert_eq!(values.len(), 2);
        assert_eq!(values, vec![10, 30]);
    }

    #[test]
    fn iterator_methods_work_with_reused_slots() {
        let mut map = SlotMap::<i32>::new();
        let key1 = map.insert(10);
        let key2 = map.insert(20);

        map.remove(key1);
        let key3 = map.insert(30); // This should reuse key1's slot

        // Check that iter only returns the valid items
        let mut items: Vec<_> = map.iter().collect();
        items.sort_by_key(|(k, _)| k.idx());

        assert_eq!(items.len(), 2);
        assert_eq!(*items[0].1, 30); // key3's value
        assert_eq!(*items[1].1, 20); // key2's value

        // Check that keys only returns valid keys
        let mut keys: Vec<_> = map.keys().collect();
        keys.sort_by_key(|k| k.idx());

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], key3);
        assert_eq!(keys[1], key2);

        // Check that values only returns valid values
        let mut values: Vec<_> = map.values().copied().collect();
        values.sort();

        assert_eq!(values.len(), 2);
        assert_eq!(values, vec![20, 30]);
    }
}
