//! Management of uniform buffers for GPU computation or rendering.

use crate::{
    geometry::{CollectionChange, CollectionChangeTracker},
    gpu::{
        rendering::buffer::{self, Count, CountedRenderBuffer, RenderBuffer, UniformBufferable},
        GraphicsDevice,
    },
};
use bytemuck::Zeroable;
use impact_utils::{ConstStringHash64, KeyIndexMapper};
use std::{
    borrow::Cow,
    fmt::Debug,
    hash::Hash,
    mem,
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

/// Render buffer for a single uniform.
#[derive(Debug)]
pub struct SingleUniformRenderBuffer {
    render_buffer: RenderBuffer,
    template_bind_group_layout_entry: wgpu::BindGroupLayoutEntry,
}

/// Render buffer for multiple uniforms of the same type.
#[derive(Debug)]
pub struct MultiUniformRenderBuffer {
    render_buffer: CountedRenderBuffer,
    uniform_type_id: ConstStringHash64,
    template_bind_group_layout_entry: wgpu::BindGroupLayoutEntry,
}

/// Indicates whether a new render buffer had to be created in order to hold all
/// the transferred uniform data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UniformTransferResult {
    CreatedNewBuffer,
    UpdatedExistingBuffer,
    NothingToTransfer,
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

    /// Creates a new empty buffer with allocated space for the given number of
    /// uniforms.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            raw_buffer: vec![U::zeroed(); capacity],
            index_map: KeyIndexMapper::new(),
            n_valid_uniforms: AtomicUsize::new(0),
            change_tracker: CollectionChangeTracker::default(),
        }
    }

    /// Returns the kind of change that has been made to the uniform buffer
    /// since the last reset of change tracing.
    pub fn change(&self) -> CollectionChange {
        self.change_tracker.change()
    }

    /// Returns the current number of valid uniforms in the buffer.
    pub fn n_valid_uniforms(&self) -> usize {
        self.n_valid_uniforms.load(Ordering::Acquire)
    }

    /// Returns a reference to the uniform with the given ID. If the entity does
    /// not have this component, [`None`] is returned.
    pub fn get_uniform(&self, uniform_id: ID) -> Option<&U> {
        self.index_map
            .get(uniform_id)
            .map(|idx| &self.raw_buffer[idx])
    }

    /// Returns a mutable reference to the uniform with the given ID. If the
    /// entity does not have this component, [`None`] is returned.
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

    /// Returns a mutable reference to the uniform with the given ID.
    ///
    /// # Panics
    /// If no uniform with the given ID exists.
    pub fn uniform_mut(&mut self, uniform_id: ID) -> &mut U {
        self.get_uniform_mut(uniform_id)
            .expect("Requested missing uniform")
    }

    /// Returns a slice with all the uniforms in the buffer, including invalid
    /// ones.
    ///
    /// # Warning
    /// Only the elements below [`n_valid_uniforms`](Self::n_valid_uniforms) are
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

    /// Inserts the given uniform identified by the given ID into the buffer.
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

impl SingleUniformRenderBuffer {
    /// Creates a new render buffer for the given uniform.
    ///
    /// # Panics
    /// If the size of the uniform is zero.
    pub fn for_uniform<U>(
        graphics_device: &GraphicsDevice,
        uniform: &U,
        visibility: wgpu::ShaderStages,
        label: Cow<'static, str>,
    ) -> Self
    where
        U: UniformBufferable,
    {
        assert_ne!(
            mem::size_of::<U>(),
            0,
            "Tried to create render resource from zero-sized uniform"
        );

        let render_buffer = RenderBuffer::new_buffer_for_single_uniform_bytes(
            graphics_device,
            bytemuck::bytes_of(uniform),
            label,
        );

        // The binding of 0 is just a placeholder, as the actual binding will be
        // assigned when calling [`Self::create_bind_group_layout_entry`]
        let template_bind_group_layout_entry = U::create_bind_group_layout_entry(0, visibility);

        Self {
            render_buffer,
            template_bind_group_layout_entry,
        }
    }

    /// Creates a bind group layout entry for the uniform.
    pub fn create_bind_group_layout_entry(&self, binding: u32) -> wgpu::BindGroupLayoutEntry {
        let mut bind_group_layout_entry = self.template_bind_group_layout_entry;
        bind_group_layout_entry.binding = binding;
        bind_group_layout_entry
    }

    /// Creates a bind group entry for the uniform.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        buffer::create_single_uniform_bind_group_entry(binding, &self.render_buffer)
    }
}

impl MultiUniformRenderBuffer {
    /// Creates a new uniform render buffer initialized from the given uniform
    /// buffer.
    pub fn for_uniform_buffer<ID, U>(
        graphics_device: &GraphicsDevice,
        uniform_buffer: &UniformBuffer<ID, U>,
        visibility: wgpu::ShaderStages,
    ) -> Self
    where
        ID: Copy + Hash + Eq + Debug,
        U: UniformBufferable,
    {
        let uniform_type_id = U::ID;

        let render_buffer = CountedRenderBuffer::new_uniform_buffer(
            graphics_device,
            uniform_buffer.raw_buffer(),
            uniform_buffer.n_valid_uniforms(),
            Cow::Borrowed(uniform_type_id.string()),
        );

        // The binding of 0 is just a placeholder, as the actual binding will be
        // assigned when calling [`Self::create_bind_group_layout_entry`]
        let template_bind_group_layout_entry = U::create_bind_group_layout_entry(0, visibility);

        Self {
            render_buffer,
            template_bind_group_layout_entry,
            uniform_type_id,
        }
    }

    /// Returns the maximum number of uniforms that can fit in the
    /// buffer.
    pub fn max_uniform_count(&self) -> usize {
        self.render_buffer.max_item_count()
    }

    /// Creates a bind group layout entry for the uniform buffer.
    pub fn create_bind_group_layout_entry(&self, binding: u32) -> wgpu::BindGroupLayoutEntry {
        let mut bind_group_layout_entry = self.template_bind_group_layout_entry;
        bind_group_layout_entry.binding = binding;
        bind_group_layout_entry
    }

    /// Creates the bind group entry for the currently valid part
    /// of the uniform buffer, assigned to the given binding.
    ///
    /// # Warning
    /// This binding will be out of date as soon as the number of
    /// valid uniforms changes.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        self.render_buffer().create_bind_group_entry(binding)
    }

    /// Returns the render buffer of uniforms.
    pub fn render_buffer(&self) -> &CountedRenderBuffer {
        &self.render_buffer
    }

    /// Writes the valid uniforms in the given uniform buffer into the uniform
    /// render buffer if the uniform buffer has changed (reallocating the render
    /// buffer if required).
    ///
    /// # Returns
    /// A [`UniformTransferResult`] indicating whether the render buffer had to
    /// be reallocated, in which case its bind group should also be recreated.
    ///
    /// # Panics
    /// If the given uniform buffer stores a different type of uniform than the
    /// render buffer.
    pub fn transfer_uniforms_to_render_buffer<ID, U>(
        &mut self,
        graphics_device: &GraphicsDevice,
        uniform_buffer: &UniformBuffer<ID, U>,
    ) -> UniformTransferResult
    where
        ID: Copy + Hash + Eq + Debug,
        U: UniformBufferable,
    {
        assert_eq!(U::ID, self.uniform_type_id);

        let change = uniform_buffer.change();

        let result = if change != CollectionChange::None {
            let valid_uniforms = uniform_buffer.valid_uniforms();
            let n_valid_uniforms = valid_uniforms.len();

            let n_valid_uniform_bytes = mem::size_of::<U>().checked_mul(n_valid_uniforms).unwrap();

            if self
                .render_buffer
                .bytes_exceed_capacity(n_valid_uniform_bytes)
            {
                // If the number of valid uniforms exceeds the capacity of the existing buffer,
                // we create a new one that is large enough for all the uniforms (also the ones
                // not currently valid)
                self.render_buffer = CountedRenderBuffer::new_uniform_buffer(
                    graphics_device,
                    uniform_buffer.raw_buffer(),
                    n_valid_uniforms,
                    self.render_buffer.label().clone(),
                );

                UniformTransferResult::CreatedNewBuffer
            } else {
                // We need to update the count of valid uniforms in the render buffer if it
                // has changed
                let new_count = if change == CollectionChange::Count {
                    Some(Count::try_from(n_valid_uniforms).unwrap())
                } else {
                    None
                };

                self.render_buffer.update_valid_bytes(
                    graphics_device,
                    bytemuck::cast_slice(valid_uniforms),
                    new_count,
                );

                UniformTransferResult::UpdatedExistingBuffer
            }
        } else {
            UniformTransferResult::NothingToTransfer
        };

        uniform_buffer.reset_change_tracking();

        result
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
