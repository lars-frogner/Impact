//! Management of uniforms.

use crate::geometry::{CollectionChange, CollectionChangeTracker};
use bytemuck::Zeroable;
use impact_utils::KeyIndexMapper;
use std::{
    fmt::Debug,
    hash::Hash,
    sync::atomic::{AtomicUsize, Ordering},
};

/// A buffer for uniforms.
///
/// The buffer is grown on demand, but never shrunk. Instead, a counter keeps
/// track of the position of the last valid uniform in the buffer, and the
/// counter is reset to zero when the buffer is cleared. This allows it to be
/// filled and emptied repeatedly without unneccesary allocations.
#[derive(Debug)]
pub struct UniformBuffer<ID, U> {
    raw_buffer: Vec<U>,
    index_map: KeyIndexMapper<ID>,
    n_valid_uniforms: AtomicUsize,
    change_tracker: CollectionChangeTracker,
}

impl<ID, U> UniformBuffer<ID, U>
where
    ID: Copy + Hash + Eq + Debug,
    U: Copy + Zeroable,
{
    /// Creates a new empty buffer for uniforms.
    pub fn new() -> Self {
        Self {
            raw_buffer: Vec::new(),
            index_map: KeyIndexMapper::new(),
            n_valid_uniforms: AtomicUsize::new(0),
            change_tracker: CollectionChangeTracker::default(),
        }
    }

    /// Creates a new empty buffer with allocated space for the
    /// given number of uniforms.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            raw_buffer: vec![U::zeroed(); capacity],
            index_map: KeyIndexMapper::new(),
            n_valid_uniforms: AtomicUsize::new(0),
            change_tracker: CollectionChangeTracker::default(),
        }
    }

    /// Returns the kind of change that has been made to the uniform
    /// buffer since the last reset of change tracing.
    pub fn change(&self) -> CollectionChange {
        self.change_tracker.change()
    }

    /// Returns the current number of valid uniforms in the buffer.
    pub fn n_valid_uniforms(&self) -> usize {
        self.n_valid_uniforms.load(Ordering::Acquire)
    }

    /// Returns a reference to the uniform with the given ID.
    /// If the entity does not have this component, [`None`]
    /// is returned.
    pub fn get_uniform(&self, uniform_id: ID) -> Option<&U> {
        self.index_map
            .get(uniform_id)
            .map(|idx| &self.raw_buffer[idx])
    }

    /// Returns a mutable reference to the uniform with the given
    /// ID. If the entity does not have this component, [`None`]
    /// is returned.
    pub fn get_uniform_mut(&mut self, uniform_id: ID) -> Option<&mut U> {
        self.change_tracker.notify_content_change();
        self.index_map
            .get(uniform_id)
            .map(|idx| &mut self.raw_buffer[idx])
    }

    /// Returns a reference to the uniform with the given ID.
    ///
    /// # Panics
    /// If no uniform with the given ID exists.
    pub fn uniform(&self, uniform_id: ID) -> &U {
        self.get_uniform(uniform_id)
            .expect("Requested missing uniform")
    }

    /// Returns a mutable reference to the uniform with the
    /// given ID.
    ///
    /// # Panics
    /// If no uniform with the given ID exists.
    pub fn uniform_mut(&mut self, uniform_id: ID) -> &mut U {
        self.get_uniform_mut(uniform_id)
            .expect("Requested missing uniform")
    }

    /// Returns a slice with all the uniforms in the buffer,
    /// including invalid ones.
    ///
    /// # Warning
    /// Only the elements below
    /// [`n_valid_uniforms`](Self::n_valid_uniforms) are
    /// considered to have valid values.
    pub fn raw_buffer(&self) -> &[U] {
        &self.raw_buffer
    }

    /// Returns a slice with the valid uniforms in the buffer.
    pub fn valid_uniforms(&self) -> &[U] {
        &self.raw_buffer[0..self.n_valid_uniforms()]
    }

    /// Returns a mutable slice with the valid uniforms in the buffer.
    pub fn valid_uniforms_mut(&mut self) -> &mut [U] {
        let n_valid_uniforms = self.n_valid_uniforms();
        &mut self.raw_buffer[0..n_valid_uniforms]
    }

    /// Returns a slice with the IDs of the valid uniforms in the buffer, in the
    /// same order as the corresponding uniforms are stored.
    pub fn valid_uniform_ids(&self) -> &[ID] {
        &self.index_map.keys_at_indices()[0..self.n_valid_uniforms()]
    }

    /// Returns an iterator over the valid uniforms where each item contains the
    /// uniform ID and a mutable reference to the uniform.
    pub fn valid_uniforms_with_ids_mut(&mut self) -> impl Iterator<Item = (ID, &'_ mut U)> {
        let n_valid_uniforms = self.n_valid_uniforms();
        let ids = &self.index_map.keys_at_indices()[0..n_valid_uniforms];
        let uniforms = &mut self.raw_buffer[0..n_valid_uniforms];
        ids.iter().copied().zip(uniforms.iter_mut())
    }

    /// Inserts the given uniform identified by the given ID
    /// into the buffer.
    ///
    /// # Panics
    /// If a uniform with the same ID already exists.
    pub fn add_uniform(&mut self, uniform_id: ID, uniform: U) {
        let buffer_length = self.raw_buffer.len();
        let idx = self.n_valid_uniforms.fetch_add(1, Ordering::SeqCst);
        assert!(idx <= buffer_length);

        // If the buffer is full, grow it first
        if idx == buffer_length {
            self.grow_buffer();
        }

        self.raw_buffer[idx] = uniform;

        self.index_map.push_key(uniform_id);

        self.change_tracker.notify_count_change();
    }

    /// Removes the uniform with the given ID.
    ///
    /// # Panics
    /// If no uniform with the given ID exists.
    pub fn remove_uniform(&mut self, uniform_id: ID) {
        let idx = self.index_map.swap_remove_key(uniform_id);
        let last_idx = self.raw_buffer.len() - 1;
        self.raw_buffer.swap(idx, last_idx);
        self.n_valid_uniforms.fetch_sub(1, Ordering::SeqCst);

        self.change_tracker.notify_count_change();
    }

    /// Forgets any recorded changes to the uniform buffer.
    pub fn reset_change_tracking(&self) {
        self.change_tracker.reset();
    }

    fn grow_buffer(&mut self) {
        let old_buffer_length = self.raw_buffer.len();

        // Add one before doubling to avoid getting stuck at zero
        let new_buffer_length = (old_buffer_length + 1).checked_mul(2).unwrap();

        let mut new_buffer = vec![U::zeroed(); new_buffer_length];
        new_buffer[0..old_buffer_length].copy_from_slice(&self.raw_buffer);

        self.raw_buffer = new_buffer;
    }
}

impl<ID, U> Default for UniformBuffer<ID, U>
where
    ID: Copy + Hash + Eq + Debug,
    U: Copy + Zeroable,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytemuck::Zeroable;

    type Id = usize;

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable)]
    struct ByteUniform(u8);

    type ByteUniformBuffer = UniformBuffer<Id, ByteUniform>;

    #[test]
    fn creating_empty_uniform_buffer_works() {
        let buffer = ByteUniformBuffer::new();

        assert_eq!(buffer.n_valid_uniforms(), 0);
        assert!(buffer.raw_buffer().is_empty());
        assert!(buffer.valid_uniforms().is_empty());
        assert!(buffer.valid_uniform_ids().is_empty());
    }

    #[test]
    fn creating_uniform_buffer_with_capacity_works() {
        let buffer = ByteUniformBuffer::with_capacity(4);

        assert_eq!(buffer.n_valid_uniforms(), 0);
        assert_eq!(buffer.raw_buffer().len(), 4);
        assert!(buffer.valid_uniforms().is_empty());
        assert!(buffer.valid_uniform_ids().is_empty());
    }

    #[test]
    #[should_panic]
    fn requesting_uniform_from_empty_uniform_buffer_fails() {
        let buffer = ByteUniformBuffer::new();
        buffer.uniform(5);
    }

    #[test]
    #[should_panic]
    fn requesting_uniform_mutably_from_empty_uniform_buffer_fails() {
        let mut buffer = ByteUniformBuffer::new();
        buffer.uniform_mut(5);
    }

    #[test]
    fn adding_one_uniform_to_uniform_buffer_works() {
        let mut buffer = ByteUniformBuffer::new();
        let id = 3;
        let uniform = ByteUniform(7);

        buffer.add_uniform(id, uniform);

        assert_eq!(buffer.n_valid_uniforms(), 1);
        assert_eq!(buffer.uniform(id), &uniform);
        assert_eq!(buffer.uniform_mut(id), &uniform);
        assert_eq!(buffer.valid_uniforms(), &[uniform]);
        assert_eq!(buffer.valid_uniform_ids(), &[id]);
    }

    #[test]
    fn adding_two_uniforms_to_uniform_buffer_works() {
        let mut buffer = ByteUniformBuffer::new();
        let id_1 = 3;
        let id_2 = 13;
        let uniform_1 = ByteUniform(7);
        let uniform_2 = ByteUniform(42);

        buffer.add_uniform(id_1, uniform_1);
        buffer.add_uniform(id_2, uniform_2);

        assert_eq!(buffer.uniform(id_1), &uniform_1);
        assert_eq!(buffer.uniform_mut(id_1), &uniform_1);
        assert_eq!(buffer.uniform(id_2), &uniform_2);
        assert_eq!(buffer.uniform_mut(id_2), &uniform_2);

        assert_eq!(buffer.n_valid_uniforms(), 2);
        assert_eq!(buffer.valid_uniforms().len(), 2);
        assert_eq!(buffer.valid_uniform_ids().len(), 2);
        assert_eq!(
            &buffer.valid_uniforms()[0],
            buffer.uniform(buffer.valid_uniform_ids()[0])
        );
        assert_eq!(
            &buffer.valid_uniforms()[1],
            buffer.uniform(buffer.valid_uniform_ids()[1])
        );
    }

    #[test]
    #[should_panic]
    fn requesting_missing_uniform_from_uniform_buffer_fails() {
        let mut buffer = ByteUniformBuffer::new();
        buffer.add_uniform(8, ByteUniform(1));
        buffer.uniform(5);
    }

    #[test]
    #[should_panic]
    fn requesting_missing_uniform_mutably_from_uniform_buffer_fails() {
        let mut buffer = ByteUniformBuffer::new();
        buffer.add_uniform(8, ByteUniform(1));
        buffer.uniform_mut(5);
    }

    #[test]
    fn removing_only_uniform_from_uniform_buffer_works() {
        let mut buffer = ByteUniformBuffer::new();
        let id = 8;
        buffer.add_uniform(id, ByteUniform(1));

        buffer.remove_uniform(id);

        assert!(buffer.get_uniform(id).is_none());
        assert_eq!(buffer.n_valid_uniforms(), 0);
        assert!(buffer.valid_uniforms().is_empty());
        assert!(buffer.valid_uniform_ids().is_empty());
    }

    #[test]
    fn removing_second_uniform_from_uniform_buffer_works() {
        let mut buffer = ByteUniformBuffer::new();
        let id_1 = 8;
        let id_2 = 0;
        let uniform_1 = ByteUniform(0);
        let uniform_2 = ByteUniform(23);

        buffer.add_uniform(id_1, uniform_1);
        buffer.add_uniform(id_2, uniform_2);

        buffer.remove_uniform(id_2);

        assert_eq!(buffer.n_valid_uniforms(), 1);
        assert_eq!(buffer.uniform(id_1), &uniform_1);
        assert_eq!(buffer.valid_uniforms(), &[uniform_1]);
        assert_eq!(buffer.valid_uniform_ids(), &[id_1]);
    }

    #[test]
    fn change_tracking_in_uniform_buffer_works() {
        let mut buffer = ByteUniformBuffer::new();
        assert_eq!(buffer.change(), CollectionChange::None);

        let id = 4;
        let uniform = ByteUniform(99);

        buffer.add_uniform(id, uniform);
        assert_eq!(buffer.change(), CollectionChange::Count);

        buffer.uniform_mut(id);
        assert_eq!(buffer.change(), CollectionChange::Count);

        buffer.reset_change_tracking();
        assert_eq!(buffer.change(), CollectionChange::None);

        buffer.uniform(id);
        assert_eq!(buffer.change(), CollectionChange::None);

        buffer.uniform_mut(id);
        assert_eq!(buffer.change(), CollectionChange::Contents);

        buffer.remove_uniform(id);
        assert_eq!(buffer.change(), CollectionChange::Count);
    }
}
