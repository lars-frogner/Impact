//! Buffering of voxel data for rendering.

use crate::{
    gpu::{buffer::GPUBuffer, indirect::DrawIndexedIndirectArgs, storage, GraphicsDevice},
    mesh::buffer::{create_vertex_buffer_layout_for_vertex, VertexBufferable},
    voxel::{
        mesh::{ChunkedVoxelObjectMesh, VoxelMeshVertex},
        ChunkedVoxelObject, VoxelObjectID,
    },
};
use std::{borrow::Cow, sync::OnceLock};

/// Owner and manager of GPU buffers for a [`ChunkedVoxelObject`].
#[derive(Debug)]
pub struct VoxelObjectGPUBufferManager {
    vertex_buffer: GPUBuffer,
    index_buffer: GPUBuffer,
    n_indices: usize,
    chunk_submesh_buffer: GPUBuffer,
    n_chunks: usize,
    indirect_argument_buffer: GPUBuffer,
    chunk_submesh_and_argument_buffer_bind_group: wgpu::BindGroup,
}

const MESH_VERTEX_BINDING_START: u32 = 10;

/// Binding location of a specific type of voxel mesh vertex attribute.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VoxelMeshVertexAttributeLocation {
    Position = MESH_VERTEX_BINDING_START,
    NormalVector = (MESH_VERTEX_BINDING_START + 1),
}

static CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> =
    OnceLock::new();

impl VoxelObjectGPUBufferManager {
    /// Creates a new manager of GPU resources for the given
    /// [`ChunkedVoxelObject`]. This involves creating the
    /// [`ChunkedVoxelObjectMesh`] for the object and initializing GPU buffers
    /// for the associated data.
    pub fn for_voxel_object(
        graphics_device: &GraphicsDevice,
        voxel_object_id: VoxelObjectID,
        voxel_object: &ChunkedVoxelObject,
    ) -> Self {
        let mesh = ChunkedVoxelObjectMesh::create(voxel_object);

        let vertex_buffer = GPUBuffer::new_full_vertex_buffer(
            graphics_device,
            mesh.vertices(),
            Cow::Owned(format!("{}", voxel_object_id)),
        );

        let index_buffer = GPUBuffer::new_full_index_buffer(
            graphics_device,
            mesh.indices(),
            Cow::Owned(format!("{}", voxel_object_id)),
        );

        let chunk_submesh_buffer = GPUBuffer::new_storage_buffer(
            graphics_device,
            mesh.chunk_submeshes(),
            Cow::Owned(format!("{} chunk info", voxel_object_id)),
        );

        let indirect_argument_buffer = GPUBuffer::new_draw_indexed_indirect_buffer(
            graphics_device,
            &vec![DrawIndexedIndirectArgs::default(); mesh.n_chunks()],
            Cow::Owned(format!("{} draw argument", voxel_object_id)),
        );

        let chunk_submesh_and_argument_buffer_bind_group =
            Self::create_submesh_and_argument_buffer_bind_group(
                graphics_device.device(),
                &chunk_submesh_buffer,
                &indirect_argument_buffer,
                Self::get_or_create_submesh_and_argument_buffer_bind_group_layout(graphics_device),
            );

        Self {
            vertex_buffer,
            index_buffer,
            n_indices: mesh.indices().len(),
            chunk_submesh_buffer,
            n_chunks: mesh.n_chunks(),
            indirect_argument_buffer,
            chunk_submesh_and_argument_buffer_bind_group,
        }
    }

    /// Return a reference to the [`GPUBuffer`] holding all the vertices in the
    /// object's mesh.
    pub fn vertex_gpu_buffer(&self) -> &GPUBuffer {
        &self.vertex_buffer
    }

    /// Return a reference to the [`GPUBuffer`] holding all the indices defining
    /// the triangles in the object's mesh.
    pub fn index_gpu_buffer(&self) -> &GPUBuffer {
        &self.index_buffer
    }

    /// Returns the format of the indices in the index buffer.
    pub fn index_format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint16
    }

    /// Returns the total number of indices in the index buffer.
    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    /// Returns the GPU buffer containing the submesh data for each chunk.
    pub fn chunk_submesh_gpu_buffer(&self) -> &GPUBuffer {
        &self.chunk_submesh_buffer
    }

    /// Returns the total number of chunks in the chunk submesh buffer.
    pub fn n_chunks(&self) -> usize {
        self.n_chunks
    }

    /// Returns the GPU buffer containing the indirect draw call arguments for
    /// each chunk.
    pub fn indirect_argument_gpu_buffer(&self) -> &GPUBuffer {
        &self.indirect_argument_buffer
    }

    /// Returns the layout of the bind group for the chunk submesh and indirect
    /// argument buffers, after creating and caching it if it has not already
    /// been created.
    pub fn get_or_create_submesh_and_argument_buffer_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT.get_or_init(|| {
            Self::create_submesh_and_argument_buffer_bind_group_layout(graphics_device.device())
        })
    }

    /// Returns a reference to the bind group for the chunk submesh and indirect
    /// argument buffers.
    pub fn submesh_and_argument_buffer_bind_group(&self) -> &wgpu::BindGroup {
        &self.chunk_submesh_and_argument_buffer_bind_group
    }

    fn create_submesh_and_argument_buffer_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        let chunk_submesh_buffer_layout = storage::create_storage_buffer_bind_group_layout_entry(
            0,
            wgpu::ShaderStages::COMPUTE,
            true,
        );

        let indirect_argument_buffer_layout =
            storage::create_storage_buffer_bind_group_layout_entry(
                1,
                wgpu::ShaderStages::COMPUTE,
                false,
            );

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[chunk_submesh_buffer_layout, indirect_argument_buffer_layout],
            label: Some("Voxel object submesh and indirect argument buffer bind group layout"),
        })
    }

    fn create_submesh_and_argument_buffer_bind_group(
        device: &wgpu::Device,
        chunk_submesh_buffer: &GPUBuffer,
        indirect_argument_buffer: &GPUBuffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                chunk_submesh_buffer.create_bind_group_entry(0),
                indirect_argument_buffer.create_bind_group_entry(1),
            ],
            label: Some("Voxel object submesh and indirect argument buffer bind group"),
        })
    }

    pub fn sync_with_voxel_object(
        &mut self,
        _graphics_device: &GraphicsDevice,
        _voxel_object: &ChunkedVoxelObject,
    ) {
    }
}

impl VertexBufferable for VoxelMeshVertex {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            VoxelMeshVertexAttributeLocation::Position as u32 => Float32x3,
            VoxelMeshVertexAttributeLocation::NormalVector as u32 => Float32x3,
        ]);
}
