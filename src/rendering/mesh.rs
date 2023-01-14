//! Management of mesh data for rendering.

use crate::{
    geometry::{CollectionChange, ColorVertex, TextureVertex, TriangleMesh},
    rendering::{
        buffer::{self, IndexBufferable, RenderBuffer, VertexBufferable},
        fre, CoreRenderingSystem, MeshShaderInput,
    },
};

/// Owner and manager of render buffers for mesh geometry.
#[derive(Debug)]
pub struct MeshRenderBufferManager {
    vertex_buffer: RenderBuffer,
    index_buffer: RenderBuffer,
    vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
    index_format: wgpu::IndexFormat,
    shader_input: MeshShaderInput,
    n_indices: usize,
    label: String,
}

impl MeshRenderBufferManager {
    /// Creates a new manager with render buffers initialized
    /// from the given mesh.
    pub fn for_mesh(
        core_system: &CoreRenderingSystem,
        mesh: &TriangleMesh<impl VertexBufferable>,
        label: String,
    ) -> Self {
        Self::new(core_system, mesh.vertices(), mesh.indices(), label)
    }

    /// Ensures that the render buffers are in sync with the given mesh.
    pub fn sync_with_mesh(
        &mut self,
        core_system: &CoreRenderingSystem,
        mesh: &TriangleMesh<impl VertexBufferable>,
    ) {
        self.sync_vertex_buffer(core_system, mesh.vertices(), mesh.vertex_change());
        self.sync_index_buffer(core_system, mesh.indices(), mesh.index_change());
        mesh.reset_vertex_index_change_tracking();
    }

    /// Returns the layout of the vertex buffer.
    pub fn vertex_buffer_layout(&self) -> &wgpu::VertexBufferLayout<'static> {
        &self.vertex_buffer_layout
    }

    /// Returns the format of the indices in the index buffer.
    pub fn index_format(&self) -> wgpu::IndexFormat {
        self.index_format
    }

    /// Returns the render buffer of vertices.
    pub fn vertex_render_buffer(&self) -> &RenderBuffer {
        &self.vertex_buffer
    }

    /// Returns the render buffer of indices.
    pub fn index_render_buffer(&self) -> &RenderBuffer {
        &self.index_buffer
    }

    /// The input required for accessing the vertex attributes
    /// in a shader.
    pub fn shader_input(&self) -> &MeshShaderInput {
        &self.shader_input
    }

    /// Returns the number of indices in the index buffer.
    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    /// Creates a new manager with a render buffer initialized
    /// from the given slices of vertices and indices.
    fn new<V, I>(
        core_system: &CoreRenderingSystem,
        vertices: &[V],
        indices: &[I],
        label: String,
    ) -> Self
    where
        V: VertexBufferable,
        I: IndexBufferable,
    {
        let vertex_buffer = RenderBuffer::new_full_vertex_buffer(core_system, vertices, &label);
        let index_buffer = RenderBuffer::new_full_index_buffer(core_system, indices, &label);
        Self {
            vertex_buffer,
            index_buffer,
            vertex_buffer_layout: V::BUFFER_LAYOUT,
            index_format: I::INDEX_FORMAT,
            shader_input: V::SHADER_INPUT,
            n_indices: indices.len(),
            label,
        }
    }

    fn sync_vertex_buffer<V>(
        &mut self,
        core_system: &CoreRenderingSystem,
        vertices: &[V],
        vertex_change: CollectionChange,
    ) where
        V: VertexBufferable,
    {
        assert_eq!(V::BUFFER_LAYOUT, self.vertex_buffer_layout);

        if vertex_change != CollectionChange::None {
            let vertex_bytes = bytemuck::cast_slice(vertices);

            if vertex_bytes.len() > self.vertex_buffer.buffer_size() {
                // If the new number of vertices exceeds the size of the existing buffer,
                // we create a new one that is large enough
                self.vertex_buffer =
                    RenderBuffer::new_full_vertex_buffer(core_system, vertices, &self.label);
            } else {
                self.vertex_buffer
                    .update_valid_bytes(core_system, vertex_bytes);
            }
        }
    }

    fn sync_index_buffer<I>(
        &mut self,
        core_system: &CoreRenderingSystem,
        indices: &[I],
        index_change: CollectionChange,
    ) where
        I: IndexBufferable,
    {
        assert_eq!(I::INDEX_FORMAT, self.index_format);

        if index_change != CollectionChange::None {
            let index_bytes = bytemuck::cast_slice(indices);

            if index_bytes.len() > self.index_buffer.buffer_size() {
                // If the new number of indices exceeds the size of the existing buffer,
                // we create a new one that is large enough
                self.index_buffer =
                    RenderBuffer::new_full_index_buffer(core_system, indices, &self.label);
            } else {
                self.index_buffer
                    .update_valid_bytes(core_system, index_bytes);
            }

            self.n_indices = indices.len();
        }
    }
}

impl VertexBufferable for ColorVertex<fre> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        buffer::create_vertex_buffer_layout_for_vertex::<Self>(
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        );

    const SHADER_INPUT: MeshShaderInput = MeshShaderInput {
        position_location: 0,
        vertex_normal_location: None,
        texture_coord_location: None,
    };
}

impl VertexBufferable for TextureVertex<fre> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        buffer::create_vertex_buffer_layout_for_vertex::<Self>(
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
        );

    const SHADER_INPUT: MeshShaderInput = MeshShaderInput {
        position_location: 0,
        vertex_normal_location: None,
        texture_coord_location: Some(1),
    };
}
