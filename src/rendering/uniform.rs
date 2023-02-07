//! Management of uniform data for rendering.

use crate::{
    geometry::{CollectionChange, UniformBuffer},
    rendering::{
        buffer::{Count, CountedRenderBuffer, UniformBufferable},
        CoreRenderingSystem,
    },
};
use impact_utils::ConstStringHash64;
use std::{borrow::Cow, fmt::Debug, hash::Hash, mem};

/// Owner and manager of a render buffer for uniforms.
#[derive(Debug)]
pub struct UniformRenderBufferManager {
    uniform_render_buffer: CountedRenderBuffer,
    uniform_id: ConstStringHash64,
}

/// Indicates whether a new render buffer had to be created in order to hold all
/// the transferred uniform data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UniformTransferResult {
    CreatedNewBuffer,
    UpdatedExistingBuffer,
    NothingToTransfer,
}

impl UniformRenderBufferManager {
    /// Creates a new manager with a render buffer initialized
    /// from the given uniform buffer.
    pub fn for_uniform_buffer<ID, U>(
        core_system: &CoreRenderingSystem,
        uniform_buffer: &UniformBuffer<ID, U>,
    ) -> Self
    where
        ID: Copy + Hash + Eq + Debug,
        U: UniformBufferable,
    {
        let uniform_id = U::ID;

        let uniform_render_buffer = CountedRenderBuffer::new_uniform_buffer(
            core_system,
            uniform_buffer.raw_buffer(),
            uniform_buffer.n_valid_uniforms(),
            Cow::Borrowed(uniform_id.string()),
        );

        Self {
            uniform_render_buffer,
            uniform_id,
        }
    }

    /// Returns the maximum number of uniforms that can fit in the
    /// buffer.
    pub fn max_uniform_count(&self) -> usize {
        self.uniform_render_buffer.max_item_count()
    }

    /// Creates the bind group entry for the currently valid part
    /// of the uniform buffer, assigned to the given binding.
    ///
    /// # Warning
    /// This binding will be out of date as soon as the number of
    /// valid uniforms changes.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        self.uniform_render_buffer()
            .create_bind_group_entry(binding)
    }

    /// Returns the render buffer of uniforms.
    pub fn uniform_render_buffer(&self) -> &CountedRenderBuffer {
        &self.uniform_render_buffer
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
        core_system: &CoreRenderingSystem,
        uniform_buffer: &UniformBuffer<ID, U>,
    ) -> UniformTransferResult
    where
        ID: Copy + Hash + Eq + Debug,
        U: UniformBufferable,
    {
        assert_eq!(U::ID, self.uniform_id);

        let change = uniform_buffer.change();

        let result = if change != CollectionChange::None {
            let valid_uniforms = uniform_buffer.valid_uniforms();
            let n_valid_uniforms = valid_uniforms.len();

            let n_valid_uniform_bytes = mem::size_of::<U>().checked_mul(n_valid_uniforms).unwrap();

            if self
                .uniform_render_buffer
                .bytes_exceed_capacity(n_valid_uniform_bytes)
            {
                // If the number of valid uniforms exceeds the capacity of the existing buffer,
                // we create a new one that is large enough for all the uniforms (also the ones
                // not currently valid)
                self.uniform_render_buffer = CountedRenderBuffer::new_uniform_buffer(
                    core_system,
                    uniform_buffer.raw_buffer(),
                    n_valid_uniforms,
                    self.uniform_render_buffer.label().clone(),
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

                self.uniform_render_buffer.update_valid_bytes(
                    core_system,
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
