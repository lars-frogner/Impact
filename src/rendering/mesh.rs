//! Management of mesh data for rendering.

use crate::{
    geometry::{
        CollectionChange, ColorVertex, Mesh, MeshInstance, MeshInstanceContainer, TextureVertex,
    },
    rendering::{
        buffer::{BufferableInstance, BufferableVertex, IndexBuffer, InstanceBuffer, VertexBuffer},
        CoreRenderingSystem,
    },
};
use std::mem;

/// Owner and manager of render data for meshes.
#[derive(Debug)]
pub struct MeshRenderDataManager {
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    label: String,
}

/// Owner and manager of render data for mesh instances.
#[derive(Debug)]
pub struct MeshInstanceRenderDataManager {
    instance_buffer: InstanceBuffer,
    label: String,
}

impl MeshRenderDataManager {
    /// Creates a new manager with render data initialized
    /// from the given mesh.
    pub fn for_mesh(
        core_system: &CoreRenderingSystem,
        mesh: &Mesh<impl BufferableVertex>,
        label: String,
    ) -> Self {
        Self::new(core_system, mesh.vertices(), mesh.indices(), label)
    }

    /// Ensures that the render data is in sync with the corresponding
    /// data in the given mesh.
    pub fn sync_with_mesh(
        &mut self,
        core_system: &CoreRenderingSystem,
        mesh: &Mesh<impl BufferableVertex>,
    ) {
        self.sync_render_data(
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

    /// Creates a new manager with render data initialized
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

    fn sync_render_data(
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

impl MeshInstanceRenderDataManager {
    /// Creates a new manager with render data initialized
    /// from the given mesh instance container.
    pub fn new(
        core_system: &CoreRenderingSystem,
        mesh_instance_container: &MeshInstanceContainer<f32>,
        label: String,
    ) -> Self {
        let n_valid_instances = u32::try_from(mesh_instance_container.n_valid_instances()).unwrap();

        let instance_buffer = InstanceBuffer::new(
            core_system,
            mesh_instance_container.instance_buffer(),
            n_valid_instances,
            &label,
        );

        Self {
            instance_buffer,
            label,
        }
    }

    /// Writes the valid instances in the given mesh instance
    /// container into the render instance buffer (reallocating
    /// the buffer if required). The mesh instance container is
    /// then cleared.
    pub fn transfer_mesh_instances_to_render_buffer(
        &mut self,
        core_system: &CoreRenderingSystem,
        mesh_instance_container: &MeshInstanceContainer<f32>,
    ) {
        let n_valid_instances = u32::try_from(mesh_instance_container.n_valid_instances()).unwrap();

        if n_valid_instances > self.instance_buffer.max_instances() {
            // Reallocate buffer since it is too small
            self.instance_buffer = InstanceBuffer::new(
                core_system,
                mesh_instance_container.instance_buffer(),
                n_valid_instances,
                &self.label,
            );
        } else {
            // Write valid instances into the beginning of the buffer
            self.instance_buffer
                .update_valid_instances(core_system, mesh_instance_container.valid_instances());
        }

        // Clear container so that it is ready for reuse
        mesh_instance_container.clear();
    }

    /// Returns the buffer of instances.
    pub fn instance_buffer(&self) -> &InstanceBuffer {
        &self.instance_buffer
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

impl BufferableVertex for MeshInstance<f32> {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4],
    };
}

impl BufferableInstance for MeshInstance<f32> {}
