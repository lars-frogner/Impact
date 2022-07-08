//! Management of mesh data for rendering.

use crate::{
    geometry::{
        CollectionChange, ColorVertex, Mesh, MeshInstance, MeshInstanceGroup, TextureVertex,
    },
    rendering::{
        buffer::{BufferableInstance, BufferableVertex, IndexBuffer, InstanceBuffer, VertexBuffer},
        CoreRenderingSystem,
    },
};
use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix4;
use std::mem;

/// Owner and manager of render data for meshes.
pub struct MeshRenderDataManager {
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    label: String,
    vertex_buffer_change: CollectionChange,
    index_buffer_change: CollectionChange,
}

/// Owner and manager of render data for mesh instances.
pub struct MeshInstanceRenderDataManager {
    instance_buffer: InstanceBuffer,
    label: String,
    instance_buffer_change: CollectionChange,
}

/// Representation of a transform for a mesh instance as a
/// nested slice of matrix elements.
///
/// Used to draw multiple versions of the same basic mesh
/// without replicating vertex and index data.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct RawMeshInstanceTransformMatrix {
    transform_matrix: [[f32; 4]; 4],
}

impl MeshRenderDataManager {
    /// Creates a new manager with render data initialized
    /// from the given mesh.
    pub fn for_mesh<V>(core_system: &CoreRenderingSystem, mesh: &Mesh<V>, label: String) -> Self
    where
        V: BufferableVertex,
    {
        Self::new(core_system, mesh.vertices(), mesh.indices(), label)
    }

    /// Ensures that the render data is in sync with the corresponding
    /// data in the given mesh.
    pub fn sync_with_mesh<V>(&mut self, core_system: &CoreRenderingSystem, mesh: &mut Mesh<V>)
    where
        V: BufferableVertex,
    {
        self.vertex_buffer_change = mesh.vertex_change();
        self.index_buffer_change = mesh.index_change();

        self.sync_render_data(
            core_system,
            mesh.vertices(),
            mesh.indices(),
            self.vertex_buffer_change,
            self.index_buffer_change,
        );
        mesh.reset_vertex_index_change_tracking();
    }

    /// Returns the kind of change that was done to the vertex
    /// buffer at the latest sync.
    pub fn vertex_buffer_change(&self) -> CollectionChange {
        self.vertex_buffer_change
    }

    /// Returns the kind of change that was done to the index
    /// buffer at the latest sync.
    pub fn index_buffer_change(&self) -> CollectionChange {
        self.index_buffer_change
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
    fn new<V>(
        core_system: &CoreRenderingSystem,
        vertices: &[V],
        indices: &[u16],
        label: String,
    ) -> Self
    where
        V: BufferableVertex,
    {
        let vertex_buffer = VertexBuffer::new(core_system, vertices, &label);
        let index_buffer = IndexBuffer::new(core_system, indices, &label);
        Self {
            vertex_buffer,
            index_buffer,
            label,
            vertex_buffer_change: CollectionChange::None,
            index_buffer_change: CollectionChange::None,
        }
    }

    fn sync_render_data<V>(
        &mut self,
        core_system: &CoreRenderingSystem,
        vertices: &[V],
        indices: &[u16],
        vertex_change: CollectionChange,
        index_change: CollectionChange,
    ) where
        V: BufferableVertex,
    {
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
    /// from the given mesh instance group.
    pub fn for_mesh_instance_group(
        core_system: &CoreRenderingSystem,
        mesh_instance_group: &MeshInstanceGroup<f32>,
        label: String,
    ) -> Self {
        Self::new(core_system, mesh_instance_group.instances(), label)
    }

    /// Ensures that the render data is in sync with the corresponding
    /// data in the given mesh instance group.
    pub fn sync_with_mesh_instance_group(
        &mut self,
        core_system: &CoreRenderingSystem,
        mesh_instance_group: &mut MeshInstanceGroup<f32>,
    ) {
        self.instance_buffer_change = mesh_instance_group.instance_change();
        self.sync_render_data(
            core_system,
            mesh_instance_group.instances(),
            self.instance_buffer_change,
        );
        mesh_instance_group.reset_instance_change_tracking();
    }

    /// Returns the kind of change that was done to the instance
    /// buffer at the latest sync.
    pub fn instance_buffer_change(&self) -> CollectionChange {
        self.instance_buffer_change
    }

    /// Returns the buffer of instances.
    pub fn instance_buffer(&self) -> &InstanceBuffer {
        &self.instance_buffer
    }

    /// Creates a new manager with render data initialized
    /// from the given slice of mesh instances.
    fn new(
        core_system: &CoreRenderingSystem,
        instances: &[MeshInstance<f32>],
        label: String,
    ) -> Self {
        let transforms = Self::create_raw_transforms(instances);
        let instance_buffer = InstanceBuffer::new(core_system, &transforms, &label);
        Self {
            instance_buffer,
            label,
            instance_buffer_change: CollectionChange::None,
        }
    }

    fn sync_render_data(
        &mut self,
        core_system: &CoreRenderingSystem,
        instances: &[MeshInstance<f32>],
        instance_change: CollectionChange,
    ) {
        match instance_change {
            CollectionChange::None => {}
            CollectionChange::Contents => {
                let transforms = Self::create_raw_transforms(instances);
                self.instance_buffer
                    .queue_update_of_instances(core_system, 0, &transforms);
            }
            CollectionChange::Count => {
                let transforms = Self::create_raw_transforms(instances);
                self.instance_buffer = InstanceBuffer::new(core_system, &transforms, &self.label);
            }
        }
    }

    fn create_raw_transforms(
        instances: &[MeshInstance<f32>],
    ) -> Vec<RawMeshInstanceTransformMatrix> {
        instances
            .iter()
            .map(RawMeshInstanceTransformMatrix::from_mesh_instance)
            .collect()
    }
}

impl RawMeshInstanceTransformMatrix {
    /// Creates a new raw transform matrix representing the transform
    /// of the given mesh instance.
    pub fn from_mesh_instance(instance: &MeshInstance<f32>) -> Self {
        Self::from_matrix(instance.transform_matrix())
    }

    /// Creates a new raw transform matrix from the given `Matrix4`.
    pub fn from_matrix(transform_matrix: &Matrix4<f32>) -> Self {
        Self::new(*transform_matrix.as_ref())
    }

    fn new(transform_matrix: [[f32; 4]; 4]) -> Self {
        Self { transform_matrix }
    }
}

impl BufferableVertex for ColorVertex {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };
}

impl BufferableVertex for TextureVertex {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
    };
}

impl BufferableVertex for RawMeshInstanceTransformMatrix {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4],
    };
}

impl BufferableInstance for RawMeshInstanceTransformMatrix {}
