//! Mesh representation of chunked voxel objects.

use crate::{
    gpu::{indirect::DrawIndexedIndirectArgs, rendering::fre},
    mesh::{FrontFaceSide, TriangleMesh},
    voxel::ChunkedVoxelObject,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{vector, Point3, UnitVector3};

/// A mesh representation of a [`ChunkedVoxelObject`]. All the vertices
/// ([`VoxelMeshVertex`]) and indices for the full object are stored together,
/// but the index buffer is laid out so that the indices defining the triangles
/// for a specific chunk are contiguous in the buffer. A list of
/// [`ChunkSubmesh`] objects mapping each chunk to its segment of the index
/// buffer is also stored. To save space, the indices in each segment are
/// defined relative to the chunk's base vertex index, which is stored in the
/// [`ChunkSubmesh`].
#[derive(Debug)]
pub struct ChunkedVoxelObjectMesh {
    vertices: Vec<VoxelMeshVertex>,
    indices: Vec<u16>,
    chunk_submeshes: Vec<ChunkSubmesh>,
}

/// A vertex in a [`ChunkedVoxelObjectMesh`].
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct VoxelMeshVertex {
    pub position: Point3<fre>,
    pub normal_vector: UnitVector3<fre>,
}

/// Metadata associating a chunk in a [`ChunkedVoxelObject`] with the segment of
/// the index buffer in the [`ChunkedVoxelObjectMesh`] that defines the
/// triangles for that chunk.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct ChunkSubmesh {
    chunk_indices: [u32; 3],
    base_vertex_index: u32,
    index_offset: u32,
    index_count: u32,
}

impl ChunkedVoxelObjectMesh {
    pub fn create(voxel_object: &ChunkedVoxelObject) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut chunk_submeshes = Vec::new();

        let chunk_extent = voxel_object.chunk_extent() as fre;

        let mut index_offset = 0;

        voxel_object.for_each_exposed_chunk(&mut |chunk| {
            let chunk_indices = chunk.chunk_indices();
            let chunk_center = vector![
                chunk_indices[0] as fre * chunk_extent + 0.5 * chunk_extent,
                chunk_indices[1] as fre * chunk_extent + 0.5 * chunk_extent,
                chunk_indices[2] as fre * chunk_extent + 0.5 * chunk_extent,
            ];

            let mesh = TriangleMesh::create_box(
                chunk_extent,
                chunk_extent,
                chunk_extent,
                FrontFaceSide::Outside,
            );

            let base_vertex_index = vertices.len();
            let index_count = mesh.n_indices();

            chunk_submeshes.push(ChunkSubmesh::new(
                chunk_indices[0],
                chunk_indices[1],
                chunk_indices[2],
                base_vertex_index,
                index_offset,
                index_count,
            ));

            vertices.reserve(mesh.n_vertices());
            vertices.extend(
                mesh.positions()
                    .iter()
                    .zip(mesh.normal_vectors().iter())
                    .map(|(position, normal_vector)| VoxelMeshVertex {
                        position: position.0 + chunk_center,
                        normal_vector: normal_vector.0,
                    }),
            );

            indices.reserve(index_count);
            indices.extend(
                mesh.indices()
                    .iter()
                    .map(|index| u16::try_from(*index).unwrap()),
            );

            index_offset += index_count;
        });

        Self {
            vertices,
            indices,
            chunk_submeshes,
        }
    }

    /// Returns a slice with all the vertices of the mesh.
    pub fn vertices(&self) -> &[VoxelMeshVertex] {
        &self.vertices
    }

    /// Returns a slice with all the indices defining the triangles of the mesh.
    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    /// Returns a slice with all the [`ChunkSubmesh`]es comprising the full
    /// mesh.
    pub fn chunk_submeshes(&self) -> &[ChunkSubmesh] {
        &self.chunk_submeshes
    }

    /// Returns the number of chunks in the voxel object that has associated
    /// triangles in the mesh.
    pub fn n_chunks(&self) -> usize {
        self.chunk_submeshes.len()
    }
}

impl ChunkSubmesh {
    /// Creates a new [`ChunkSubmesh`] associating the chunk at the given
    /// indices in the voxel object's chunk grid with the given index range in
    /// the index buffer of the [`ChunkedVoxelObjectMesh`].
    fn new(
        chunk_i: usize,
        chunk_j: usize,
        chunk_k: usize,
        base_vertex_index: usize,
        index_offset: usize,
        index_count: usize,
    ) -> Self {
        let chunk_i = u32::try_from(chunk_i).unwrap();
        let chunk_j = u32::try_from(chunk_j).unwrap();
        let chunk_k = u32::try_from(chunk_k).unwrap();
        let base_vertex_index = u32::try_from(base_vertex_index).unwrap();
        let index_offset = u32::try_from(index_offset).unwrap();
        let index_count = u32::try_from(index_count).unwrap();

        Self {
            chunk_indices: [chunk_i, chunk_j, chunk_k],
            base_vertex_index,
            index_offset,
            index_count,
        }
    }
}
