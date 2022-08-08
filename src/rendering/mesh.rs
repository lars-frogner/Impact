//! Management of mesh data for rendering.

use crate::{
    geometry::{CollectionChange, ColorVertex, Mesh, TextureVertex},
    rendering::{
        buffer::{BufferableVertex, IndexBuffer, VertexBuffer},
        CoreRenderingSystem,
    },
};
use std::mem;

/// Owner and manager of render buffers for mesh geometry.
#[derive(Debug)]
pub struct MeshRenderBufferManager {
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    label: String,
}

impl MeshRenderBufferManager {
    /// Creates a new manager with render buffers initialized
    /// from the given mesh.
    pub fn for_mesh(
        core_system: &CoreRenderingSystem,
        mesh: &Mesh<impl BufferableVertex>,
        label: String,
    ) -> Self {
        Self::new(core_system, mesh.vertices(), mesh.indices(), label)
    }

    /// Ensures that the render buffers are in sync with the given mesh.
    pub fn sync_with_mesh(
        &mut self,
        core_system: &CoreRenderingSystem,
        mesh: &Mesh<impl BufferableVertex>,
    ) {
        self.sync_render_buffers(
            core_system,
            mesh.vertices(),
            mesh.indices(),
            mesh.vertex_change(),
            mesh.index_change(),
        );
        mesh.reset_vertex_index_change_tracking();
    }

    /// Returns the buffer of vertices.
    pub fn vertex_buffer(&self) -> &VertexBuffer {
        &self.vertex_buffer
    }

    /// Returns the buffer of indices.
    pub fn index_buffer(&self) -> &IndexBuffer {
        &self.index_buffer
    }

    /// Creates a new manager with a render buffer initialized
    /// from the given slices of vertices and indices.
    fn new(
        core_system: &CoreRenderingSystem,
        vertices: &[impl BufferableVertex],
        indices: &[u16],
        label: String,
    ) -> Self {
        let vertex_buffer = VertexBuffer::new(core_system, vertices, &label);
        let index_buffer = IndexBuffer::new(core_system, indices, &label);
        Self {
            vertex_buffer,
            index_buffer,
            label,
        }
    }

    fn sync_render_buffers(
        &mut self,
        core_system: &CoreRenderingSystem,
        vertices: &[impl BufferableVertex],
        indices: &[u16],
        vertex_change: CollectionChange,
        index_change: CollectionChange,
    ) {
        match vertex_change {
            CollectionChange::None => {}
            CollectionChange::Contents => {
                // If the contents of the buffer needs to be updated,
                // we queue a write of the new vertices into the buffer
                self.vertex_buffer
                    .queue_update_of_vertices(core_system, 0, vertices);
            }
            CollectionChange::Count => {
                // If the size of the buffer has changed, we simply
                // rebuild the buffer
                self.vertex_buffer = VertexBuffer::new(core_system, vertices, &self.label);
            }
        }
        match index_change {
            CollectionChange::None => {}
            CollectionChange::Contents => {
                self.index_buffer
                    .queue_update_of_indices(core_system, 0, indices);
            }
            CollectionChange::Count => {
                self.index_buffer = IndexBuffer::new(core_system, indices, &self.label);
            }
        }
    }
}

impl BufferableVertex for ColorVertex<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };
}

impl BufferableVertex for TextureVertex<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
    };
}
