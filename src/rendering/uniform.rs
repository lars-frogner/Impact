//! Management of uniform data for rendering.

use crate::{
    geometry::{CollectionChange, UniformBuffer},
    rendering::{
        buffer::{BufferableUniform, DynamicUniformRenderBuffer},
        CoreRenderingSystem,
    },
};
use std::{fmt::Debug, hash::Hash, marker::PhantomData};

/// Owner and manager of a render buffer for uniforms.
#[derive(Debug)]
pub struct UniformRenderBufferManager<U> {
    uniform_render_buffer: DynamicUniformRenderBuffer,
    _phantom: PhantomData<U>,
}

impl<U> UniformRenderBufferManager<U>
where
    U: BufferableUniform,
{
    /// Creates a new manager with a render buffer initialized
    /// from the given uniform buffer.
    pub fn new<ID>(core_system: &CoreRenderingSystem, uniform_buffer: &UniformBuffer<ID, U>) -> Self
    where
        ID: Copy + Hash + Eq + Debug,
    {
        let n_valid_uniforms = u32::try_from(uniform_buffer.n_valid_uniforms()).unwrap();

        let uniform_render_buffer = DynamicUniformRenderBuffer::new(
            core_system,
            uniform_buffer.raw_buffer(),
            n_valid_uniforms,
        );

        Self {
            uniform_render_buffer,
            _phantom: PhantomData,
        }
    }

    /// Creates the bind group entry for the uniform buffer,
    /// based on the given layout.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        self.uniform_render_buffer.create_bind_group_entry(binding)
    }

    /// Writes the valid uniforms in the given uniform
    /// buffer into the uniform render buffer if the uniform
    /// buffer has changed (reallocating  the render buffer
    /// if required).
    pub fn transfer_uniforms_to_render_buffer<ID>(
        &mut self,
        core_system: &CoreRenderingSystem,
        uniform_buffer: &UniformBuffer<ID, U>,
    ) where
        ID: Copy + Hash + Eq + Debug,
    {
        if uniform_buffer.change() == CollectionChange::None {
            let n_valid_uniforms = u32::try_from(uniform_buffer.n_valid_uniforms()).unwrap();

            if n_valid_uniforms > self.uniform_render_buffer.max_uniforms() {
                // Reallocate render buffer since it is too small
                self.uniform_render_buffer = DynamicUniformRenderBuffer::new(
                    core_system,
                    uniform_buffer.raw_buffer(),
                    n_valid_uniforms,
                );
            } else {
                // Write valid uniforms into the beginning of the render buffer
                self.uniform_render_buffer
                    .update_valid_uniforms(core_system, uniform_buffer.valid_uniforms());
            }
        }
        uniform_buffer.reset_change_tracking();
    }

    /// Returns the render buffer of uniforms.
    pub fn uniform_render_buffer(&self) -> &DynamicUniformRenderBuffer {
        &self.uniform_render_buffer
    }
}
