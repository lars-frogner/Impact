//! Management of uniforms.

use crate::geometry::{CollectionChange, CollectionChangeTracker};
use bytemuck::Zeroable;
use impact_ecs::util::KeyIndexMapper;
use std::{
    fmt::Debug,
    hash::Hash,
    sync::atomic::{AtomicUsize, Ordering},
};

/// A buffer for uniforms.
///
/// The buffer is grown on demand, but never shrunk.
/// Instead, a counter keeps track of the position
/// of the last valid uniform in the buffer, and the
/// counter is reset to zero when the buffer is cleared.
/// This allows the it to be filled and emptied
/// repeatedly without unneccesary allocations.
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
