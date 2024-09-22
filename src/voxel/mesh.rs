//! Mesh representation of chunked voxel objects.

use crate::{
    geometry::{Frustum, OrientedBox, Plane},
    gpu::rendering::fre,
    voxel::chunks::{sdf::surface_nets::SurfaceNetsBuffer, ChunkedVoxelObject, VoxelChunkFlags},
};
use bytemuck::{Pod, Zeroable};
use glam::Vec3A;
use nalgebra::{Point3, Similarity3, UnitVector3};
use std::array;

/// A mesh representation of a [`ChunkedVoxelObject`]. All the vertices and
/// indices for the full object are stored together, but the index buffer is
/// laid out so that the indices defining the triangles for a specific chunk are
/// contiguous in the buffer. A list of [`ChunkSubmesh`] objects mapping each
/// chunk to its segment of the index buffer is also stored.
#[derive(Debug)]
pub struct ChunkedVoxelObjectMesh {
    positions: Vec<VoxelMeshVertexPosition>,
    normal_vectors: Vec<VoxelMeshVertexNormalVector>,
    index_materials: Vec<VoxelMeshIndexMaterials>,
    indices: Vec<VoxelMeshIndex>,
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

/// A set of four material indices and corresponding weights for a vertex index
/// in a [`ChunkedVoxelObjectMesh`]. The materials must be specificed per index
/// rather than per vertex to ensure that the four materials to blend are the
/// same for each triangle. The material indices represent the four materials
/// that have the strongest influence on the triangle containing this vertex
/// index, and the weight for the material is the number of voxels among the
/// eight voxels defining the vertex that have that material.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct VoxelMeshIndexMaterials {
    pub indices: [u8; 4],
    pub weights: [u8; 4],
}

/// A vertex index a [`ChunkedVoxelObjectMesh`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct VoxelMeshIndex(pub u32);

/// Metadata associating a chunk in a [`ChunkedVoxelObject`] with the segment of
/// the index buffer in the [`ChunkedVoxelObjectMesh`] that defines the
/// triangles for that chunk.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct ChunkSubmesh {
    chunk_indices: [u32; 3],
    index_offset: u32,
    index_count: u32,
    /// Table of booleans (stored as `u32`s to make it directly representable in
    /// WGSL) indicating whether the chunk is obscured from a specific
    /// axis-aligned direction. Used for culling obscured chunks given a view
    /// direction.
    is_obscured_from_direction: [[[u32; 2]; 2]; 2],
}

/// A set of planes defining a frustum together with a small lookup table for
/// fast culling and an apex position for computing view directions, gathered in
/// a representation suitable for passing to the GPU.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct CullingFrustum {
    pub planes: [FrustumPlane; 6],
    pub largest_signed_dist_aab_corner_indices_for_planes: [u32; 6],
    pub apex_position: Point3<fre>,
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
        let mut index_materials = Vec::new();
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

            let base_vertex_index = positions.len() as u32;
            let index_count = buffer.indices.len();

            chunk_submeshes.push(ChunkSubmesh::new(
                chunk_indices[0],
                chunk_indices[1],
                chunk_indices[2],
                index_offset,
                index_count,
                chunk.flags(),
            ));

            positions.extend_from_slice(&buffer.positions);
            normal_vectors.extend_from_slice(&buffer.normal_vectors);
            index_materials.extend_from_slice(&buffer.index_materials);

            indices.reserve(index_count);
            indices.extend(
                buffer
                    .indices
                    .iter()
                    .map(|&index| VoxelMeshIndex(base_vertex_index + u32::from(index))),
            );

            index_offset += index_count;
        });

        Self {
            positions,
            normal_vectors,
            index_materials,
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

    /// Returns a slice with the materials for each vertex index in
    /// [`Self::indices`].
    pub fn index_materials(&self) -> &[VoxelMeshIndexMaterials] {
        &self.index_materials
    }

    /// Returns a slice with all the indices defining the triangles of the mesh.
    pub fn indices(&self) -> &[VoxelMeshIndex] {
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
        index_offset: usize,
        index_count: usize,
        flags: VoxelChunkFlags,
    ) -> Self {
        let chunk_i = u32::try_from(chunk_i).unwrap();
        let chunk_j = u32::try_from(chunk_j).unwrap();
        let chunk_k = u32::try_from(chunk_k).unwrap();
        let index_offset = u32::try_from(index_offset).unwrap();
        let index_count = u32::try_from(index_count).unwrap();
        let is_obscured_from_direction = Self::compute_directional_obscuredness_table(flags);

        Self {
            chunk_indices: [chunk_i, chunk_j, chunk_k],
            index_offset,
            index_count,
            is_obscured_from_direction,
        }
    }

    fn compute_directional_obscuredness_table(flags: VoxelChunkFlags) -> [[[u32; 2]; 2]; 2] {
        const OBSCURED_X: [VoxelChunkFlags; 2] = [
            VoxelChunkFlags::IS_OBSCURED_X_DN,
            VoxelChunkFlags::IS_OBSCURED_X_UP,
        ];
        const OBSCURED_Y: [VoxelChunkFlags; 2] = [
            VoxelChunkFlags::IS_OBSCURED_Y_DN,
            VoxelChunkFlags::IS_OBSCURED_Y_UP,
        ];
        const OBSCURED_Z: [VoxelChunkFlags; 2] = [
            VoxelChunkFlags::IS_OBSCURED_Z_DN,
            VoxelChunkFlags::IS_OBSCURED_Z_UP,
        ];

        array::from_fn(|i| {
            let obscured_x = flags.contains(OBSCURED_X[i]);
            array::from_fn(|j| {
                let obscured_y = flags.contains(OBSCURED_Y[j]);
                array::from_fn(|k| {
                    let obscured_z = flags.contains(OBSCURED_Z[k]);
                    u32::from(obscured_x && obscured_y && obscured_z)
                })
            })
        })
    }
}

impl CullingFrustum {
    /// Gathers the given frustum planes and apex position into a
    /// `CullingFrustum`.
    pub fn from_planes_and_apex_position(
        planes: [Plane<fre>; 6],
        apex_position: Point3<fre>,
    ) -> Self {
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
            apex_position,
        }
    }

    /// Transforms the given frustum with the given similarity transform and
    /// gathers the resulting frustum planes into a `CullingFrustum`.
    ///
    /// The frustum is assumed to be in the space where the apex is at the
    /// origin before transformation.
    pub fn for_transformed_frustum(
        frustum: &Frustum<fre>,
        transformation: &Similarity3<fre>,
    ) -> Self {
        Self::from_planes_and_apex_position(
            frustum.transformed_planes(transformation),
            transformation.isometry.translation.vector.into(),
        )
    }

    /// Transforms the given orthographic frustum (represented by an oriented
    /// box) with the given similarity transform and gathers the resulting
    /// frustum planes into a `CullingFrustum`.
    ///
    /// The frustum is assumed to be in the space where the view direction is
    /// along the negative depth-axis of the box before transformation. An apex
    /// position is computed based on this direction and the given distance
    /// from the center of the box (the distance is assumed positive and
    /// given in the transformed space). While the apex is technically at
    /// infinity for an orthographic frustum, this can be emulated by
    /// passing in a sufficiently large distance.
    pub fn for_transformed_orthographic_frustum(
        orthographic_frustum: &OrientedBox<fre>,
        transformation: &Similarity3<fre>,
        apex_distance: fre,
    ) -> Self {
        let transformed_box = orthographic_frustum.transformed(transformation);
        let transformed_view_diection = -transformed_box.compute_depth_axis();
        let transformed_apex_position =
            transformed_box.center() - transformed_view_diection.scale(apex_distance);
        Self::from_planes_and_apex_position(
            transformed_box.compute_bounding_planes(),
            transformed_apex_position,
        )
    }
}
