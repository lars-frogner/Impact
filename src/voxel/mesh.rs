//! Mesh representation of chunked voxel objects.

use crate::{
    geometry::{Frustum, OrientedBox, Plane},
    gpu::rendering::fre,
    voxel::chunks::{sdf::surface_nets::SurfaceNetsBuffer, ChunkedVoxelObject},
};
use bytemuck::{Pod, Zeroable};
use glam::Vec3A;
use nalgebra::{Similarity3, UnitVector3};

/// A mesh representation of a [`ChunkedVoxelObject`]. All the vertices and
/// indices for the full object are stored together, but the index buffer is
/// laid out so that the indices defining the triangles for a specific chunk are
/// contiguous in the buffer. A list of [`ChunkSubmesh`] objects mapping each
/// chunk to its segment of the index buffer is also stored. To save space, the
/// indices in each segment are defined relative to the chunk's base vertex
/// index, which is stored in the [`ChunkSubmesh`].
#[derive(Debug)]
pub struct ChunkedVoxelObjectMesh {
    positions: Vec<VoxelMeshVertexPosition>,
    normal_vectors: Vec<VoxelMeshVertexNormalVector>,
    indices: Vec<u16>,
    chunk_submeshes: Vec<ChunkSubmesh>,
}

/// A vertex position in a [`ChunkedVoxelObjectMesh`].
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct VoxelMeshVertexPosition(pub [f32; 3]);

/// A vertex normal vector in a [`ChunkedVoxelObjectMesh`].
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct VoxelMeshVertexNormalVector(pub [f32; 3]);

/// Metadata associating a chunk in a [`ChunkedVoxelObject`] with the segment of
/// the index buffer in the [`ChunkedVoxelObjectMesh`] that defines the
/// triangles for that chunk.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct ChunkSubmesh {
    pub chunk_indices: [u32; 3],
    base_vertex_index: u32,
    index_offset: u32,
    index_count: u32,
}

/// A set of planes defining a frustum together with a small lookup table for
/// fast culling, gathered in a representation suitable for passing to the GPU.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct FrustumPlanes {
    pub planes: [FrustumPlane; 6],
    pub largest_signed_dist_aab_corner_indices_for_planes: [u32; 6],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct FrustumPlane {
    pub unit_normal: UnitVector3<fre>,
    pub displacement: fre,
}

impl ChunkedVoxelObjectMesh {
    pub fn create(voxel_object: &ChunkedVoxelObject) -> Self {
        let mut positions = Vec::new();
        let mut normal_vectors = Vec::new();
        let mut indices = Vec::new();
        let mut chunk_submeshes = Vec::new();

        let mut buffer = SurfaceNetsBuffer::default();

        let voxel_extent = voxel_object.voxel_extent() as fre;
        let chunk_extent = voxel_object.chunk_extent() as fre;

        let mut index_offset = 0;

        voxel_object.for_each_exposed_chunk_with_sdf(&mut |chunk, sdf| {
            let chunk_indices = chunk.chunk_indices();

            // Since the `VoxelChunkSignedDistanceField` has a 1-voxel padding
            // around the chunk boundary, we need to subtract the voxel extent
            // from the position of the chunk's lower corner to get the offset
            // of the vertices for the surface nets mesh.
            let vertex_position_offset = Vec3A::new(
                chunk_indices[0] as fre * chunk_extent - voxel_extent,
                chunk_indices[1] as fre * chunk_extent - voxel_extent,
                chunk_indices[2] as fre * chunk_extent - voxel_extent,
            );

            sdf.compute_surface_nets_mesh(voxel_extent, &vertex_position_offset, &mut buffer);

            let base_vertex_index = positions.len();
            let index_count = buffer.indices.len();

            chunk_submeshes.push(ChunkSubmesh::new(
                chunk_indices[0],
                chunk_indices[1],
                chunk_indices[2],
                base_vertex_index,
                index_offset,
                index_count,
            ));

            positions.extend_from_slice(&buffer.positions);
            normal_vectors.extend_from_slice(&buffer.normal_vectors);
            indices.extend_from_slice(&buffer.indices);

            index_offset += index_count;
        });

        Self {
            positions,
            normal_vectors,
            indices,
            chunk_submeshes,
        }
    }

    /// Returns a slice with the positions of all the vertices of the mesh.
    pub fn positions(&self) -> &[VoxelMeshVertexPosition] {
        &self.positions
    }

    /// Returns a slice with the normal vectors of all the vertices of the mesh.
    pub fn normal_vectors(&self) -> &[VoxelMeshVertexNormalVector] {
        &self.normal_vectors
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

impl FrustumPlanes {
    /// Gathers the given frustum planes into a `FrustumPlanes`.
    pub fn from_planes(planes: [Plane<fre>; 6]) -> Self {
        let largest_signed_dist_aab_corner_indices_for_planes = planes.clone().map(|plane| {
            u32::try_from(Frustum::determine_largest_signed_dist_aab_corner_index_for_plane(&plane))
                .unwrap()
        });
        let planes = planes.map(|plane| {
            let (unit_normal, displacement) = plane.into_normal_and_displacement();
            FrustumPlane {
                unit_normal,
                displacement,
            }
        });
        Self {
            planes,
            largest_signed_dist_aab_corner_indices_for_planes,
        }
    }

    /// Transforms the given frustum with the given similarity transform and
    /// gathers the resulting frustum planes into a `FrustumPlanes`.
    pub fn for_transformed_frustum(
        frustum: &Frustum<fre>,
        transformation: &Similarity3<fre>,
    ) -> Self {
        Self::from_planes(frustum.transformed_planes(transformation))
    }

    /// Transforms the given orthographic frustum (represented by an oriented
    /// box) with the given similarity transform and gathers the resulting
    /// frustum planes into a `FrustumPlanes`.
    pub fn for_transformed_orthographic_frustum(
        orthographic_frustum: &OrientedBox<fre>,
        transformation: &Similarity3<fre>,
    ) -> Self {
        let transformed_box = orthographic_frustum.transformed(transformation);
        Self::from_planes(transformed_box.compute_bounding_planes())
    }
}
