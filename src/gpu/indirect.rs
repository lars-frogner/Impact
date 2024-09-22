//! Indirect draw calls.

use crate::gpu::{
    buffer::{GPUBuffer, GPUBufferType},
    GraphicsDevice,
};
use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;

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
            GPUBufferType::Indirect,
            bytes,
            bytes.len(),
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
            GPUBufferType::Indirect,
            bytes,
            bytes.len(),
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
            GPUBufferType::Indirect,
            bytes,
            bytes.len(),
            label,
        )
    }
}
