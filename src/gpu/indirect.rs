//! Indirect draw calls.

use crate::gpu::{
    buffer::{GPUBuffer, GPUBufferType},
    GraphicsDevice,
};
use bytemuck::{Pod, Zeroable};
use std::{borrow::Cow, mem};

/// Argument buffer layout for `draw_indirect` commands.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
pub struct DrawIndirectArgs {
    /// The number of vertices to draw.
    pub vertex_count: u32,
    /// The number of instances to draw.
    pub instance_count: u32,
    /// The Index of the first vertex to draw.
    pub first_vertex: u32,
    /// The instance ID of the first instance to draw.
    ///
    /// Has to be 0, unless
    /// [`Features::INDIRECT_FIRST_INSTANCE`](crate::Features::INDIRECT_FIRST_INSTANCE)
    /// is enabled.
    pub first_instance: u32,
}

/// Argument buffer layout for `draw_indexed_indirect` commands.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod)]
pub struct DrawIndexedIndirectArgs {
    /// The number of indices to draw.
    pub index_count: u32,
    /// The number of instances to draw.
    pub instance_count: u32,
    /// The first index within the index buffer.
    pub first_index: u32,
    /// The value added to the vertex index before indexing into the vertex
    /// buffer.
    pub base_vertex: i32,
    /// The instance ID of the first instance to draw.
    ///
    /// Has to be 0, unless
    /// [`Features::INDIRECT_FIRST_INSTANCE`](crate::Features::INDIRECT_FIRST_INSTANCE)
    /// is enabled.
    pub first_instance: u32,
}

impl GPUBuffer {
    /// Creates a new GPU buffer for draw call arguments for use with
    /// [`wgpu::RenderPass::draw_indirect`],
    /// [`wgpu::RenderPass::multi_draw_indirect`] or
    /// [`wgpu::RenderPass::multi_draw_indirect_count`].
    pub fn new_draw_indirect_buffer(
        graphics_device: &GraphicsDevice,
        indirect_draw_args: &[DrawIndirectArgs],
        label: Cow<'static, str>,
    ) -> Self {
        let bytes = bytemuck::cast_slice(indirect_draw_args);
        Self::new(
            graphics_device,
            bytes,
            bytes.len(),
            GPUBufferType::Indirect.usage(),
            label,
        )
    }

    /// Creates a new GPU buffer for draw call arguments for use with
    /// [`wgpu::RenderPass::draw_indirect`],
    /// [`wgpu::RenderPass::multi_draw_indirect`] or
    /// [`wgpu::RenderPass::multi_draw_indirect_count`]. The buffer has room for
    /// the given number of argument objects, and the beginning of the buffer is
    /// initialized with the given slice of argument objects.
    pub fn new_draw_indirect_buffer_with_spare_capacity(
        graphics_device: &GraphicsDevice,
        total_indirect_draw_arg_capacity: usize,
        initial_indirect_draw_args: &[DrawIndirectArgs],
        label: Cow<'static, str>,
    ) -> Self {
        let buffer_size = mem::size_of::<DrawIndirectArgs>()
            .checked_mul(total_indirect_draw_arg_capacity)
            .unwrap();
        let valid_bytes = bytemuck::cast_slice(initial_indirect_draw_args);
        Self::new_with_spare_capacity(
            graphics_device,
            buffer_size,
            valid_bytes,
            GPUBufferType::Indirect.usage(),
            label,
        )
    }

    /// Creates a new GPU buffer for draw call arguments for use with
    /// [`wgpu::RenderPass::draw_indexed_indirect`],
    /// [`wgpu::RenderPass::multi_draw_indexed_indirect`] or
    /// [`wgpu::RenderPass::multi_draw_indexed_indirect_count`].
    pub fn new_draw_indexed_indirect_buffer(
        graphics_device: &GraphicsDevice,
        indirect_draw_args: &[DrawIndexedIndirectArgs],
        label: Cow<'static, str>,
    ) -> Self {
        let bytes = bytemuck::cast_slice(indirect_draw_args);
        Self::new(
            graphics_device,
            bytes,
            bytes.len(),
            GPUBufferType::Indirect.usage(),
            label,
        )
    }

    /// Creates a new GPU buffer for draw call arguments for use with
    /// [`wgpu::RenderPass::draw_indexed_indirect`],
    /// [`wgpu::RenderPass::multi_draw_indexed_indirect`] or
    /// [`wgpu::RenderPass::multi_draw_indexed_indirect_count`]. The buffer has
    /// room for the given number of argument objects, and the beginning of
    /// the buffer is initialized with the given slice of argument objects.
    pub fn new_draw_indexed_indirect_buffer_with_spare_capacity(
        graphics_device: &GraphicsDevice,
        total_indirect_draw_arg_capacity: usize,
        initial_indirect_draw_args: &[DrawIndexedIndirectArgs],
        label: Cow<'static, str>,
    ) -> Self {
        let buffer_size = mem::size_of::<DrawIndexedIndirectArgs>()
            .checked_mul(total_indirect_draw_arg_capacity)
            .unwrap();
        let valid_bytes = bytemuck::cast_slice(initial_indirect_draw_args);
        Self::new_with_spare_capacity(
            graphics_device,
            buffer_size,
            valid_bytes,
            GPUBufferType::Indirect.usage(),
            label,
        )
    }

    /// Creates a new GPU buffer for a draw call count for use with
    /// [`wgpu::RenderPass::multi_draw_indirect_count`] or
    /// [`wgpu::RenderPass::multi_draw_indexed_indirect_count`].
    pub fn new_multi_draw_indirect_count_buffer(
        graphics_device: &GraphicsDevice,
        count: u32,
        label: Cow<'static, str>,
    ) -> Self {
        let bytes = bytemuck::bytes_of(&count);
        Self::new(
            graphics_device,
            bytes,
            bytes.len(),
            GPUBufferType::Indirect.usage(),
            label,
        )
    }
}
