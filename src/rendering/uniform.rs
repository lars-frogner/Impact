//! Management of uniform data for rendering.

use crate::{
    geometry::{CollectionChange, UniformBuffer},
    hash::ConstStringHash,
    rendering::{
        buffer::{self, RenderBuffer, RenderBufferType, UniformBufferable},
        CoreRenderingSystem,
    },
};
use std::{fmt::Debug, hash::Hash, mem};

/// Owner and manager of a render buffer for uniforms.
#[derive(Debug)]
pub struct UniformRenderBufferManager {
    uniform_render_buffer: RenderBuffer,
    uniform_id: ConstStringHash,
}

impl UniformRenderBufferManager {
    /// Creates a new manager with a render buffer initialized
    /// from the given uniform buffer.
    pub fn new<ID, U>(
        core_system: &CoreRenderingSystem,
        uniform_buffer: &UniformBuffer<ID, U>,
    ) -> Self
    where
        ID: Copy + Hash + Eq + Debug,
        U: UniformBufferable,
    {
        let uniform_id = U::ID;

        let n_valid_bytes = mem::size_of::<U>()
            .checked_mul(uniform_buffer.n_valid_uniforms())
            .unwrap();

        let uniform_render_buffer = RenderBuffer::new(
            core_system,
            RenderBufferType::Uniform,
            bytemuck::cast_slice(uniform_buffer.raw_buffer()),
            n_valid_bytes,
            uniform_id.as_ref(),
        );

        Self {
            uniform_render_buffer,
            uniform_id,
        }
    }

    /// Creates the bind group entry for the uniform buffer,
    /// assigned to the given binding.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        buffer::create_uniform_buffer_bind_group_entry(binding, self.uniform_render_buffer())
    }

    /// Returns the render buffer of uniforms.
    pub fn uniform_render_buffer(&self) -> &RenderBuffer {
        &self.uniform_render_buffer
    }

    /// Writes the valid uniforms in the given uniform
    /// buffer into the uniform render buffer if the uniform
    /// buffer has changed (reallocating the render buffer
    /// if required).
    ///
    /// # Panics
    /// If the given uniform buffer stores a different type
    /// of uniform than the render buffer.
    pub fn transfer_uniforms_to_render_buffer<ID, U>(
        &mut self,
        core_system: &CoreRenderingSystem,
        uniform_buffer: &UniformBuffer<ID, U>,
    ) where
        ID: Copy + Hash + Eq + Debug,
        U: UniformBufferable,
    {
        assert_eq!(U::ID, self.uniform_id);

        if uniform_buffer.change() != CollectionChange::None {
            let valid_bytes = bytemuck::cast_slice(uniform_buffer.valid_uniforms());
            let n_valid_bytes = valid_bytes.len();

            if n_valid_bytes > self.uniform_render_buffer.buffer_size() {
                // If the number of valid uniforms exceeds the capacity of the existing buffer,
                // we create a new one that is large enough for all the uniforms (also the ones
                // not currently valid)
                self.uniform_render_buffer = RenderBuffer::new(
                    core_system,
                    RenderBufferType::Uniform,
                    bytemuck::cast_slice(uniform_buffer.raw_buffer()),
                    n_valid_bytes,
                    self.uniform_id.as_ref(),
                );
            } else {
                self.uniform_render_buffer
                    .update_valid_bytes(core_system, valid_bytes);
            }
        }
        uniform_buffer.reset_change_tracking();
    }
}
