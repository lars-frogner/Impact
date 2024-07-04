//! Management of uniform data for rendering.

use crate::{
    geometry::{CollectionChange, UniformBuffer},
    gpu::{
        rendering::buffer::{self, Count, CountedRenderBuffer, RenderBuffer, UniformBufferable},
        GraphicsDevice,
    },
};
use impact_utils::ConstStringHash64;
use std::{borrow::Cow, fmt::Debug, hash::Hash, mem};

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
