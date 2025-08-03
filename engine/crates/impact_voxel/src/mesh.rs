//! Mesh representation of chunked voxel objects.

use crate::chunks::{
    ChunkedVoxelObject, VoxelChunkFlags,
    sdf::{VoxelChunkSignedDistanceField, surface_nets::SurfaceNetsBuffer},
};
use bytemuck::{Pod, Zeroable};
use glam::Vec3A;
use impact_containers::KeyIndexMapper;
use impact_geometry::{Frustum, OrientedBox, Plane};
use nalgebra::{Point3, Similarity3, UnitVector3};
use std::{array, collections::BTreeSet, ops::Range};

/// A [`ChunkedVoxelObject`] with an associated [`ChunkedVoxelObjectMesh`].
#[derive(Debug)]
pub struct MeshedChunkedVoxelObject {
    object: ChunkedVoxelObject,
    mesh: ChunkedVoxelObjectMesh,
}

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
    sdf_buffer: VoxelChunkSignedDistanceField,
    surface_nets_buffer: SurfaceNetsBuffer,
    chunk_submesh_manager: ChunkSubmeshManager,
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

/// Ranges for the vertex and index data of a chunk submesh in the mesh data
/// buffer.
#[derive(Clone, Debug)]
pub struct ChunkSubmeshDataRanges {
    pub vertex_range: Range<usize>,
    pub index_range: Range<usize>,
}

/// Modifcations that were made to the voxel mesh since it was last synchronized
/// with the GPU.
#[derive(Clone, Debug)]
pub struct VoxelMeshModifications<'a> {
    /// The chunk submesh data ranges that have been updated with new data.
    pub updated_chunk_submesh_data_ranges: &'a [ChunkSubmeshDataRanges],
    /// Whether any chunks were removed as opposed to updated.
    pub chunks_were_removed: bool,
}

/// A set of planes defining a frustum together with a small lookup table for
/// fast culling and an apex position for computing view directions, gathered in
/// a representation suitable for passing to the GPU.
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct CullingFrustum {
    pub planes: [FrustumPlane; 6],
    pub largest_signed_dist_aab_corner_indices_for_planes: [u32; 6],
    pub apex_position: Point3<f32>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct FrustumPlane {
    pub unit_normal: UnitVector3<f32>,
    pub displacement: f32,
}

#[derive(Clone, Debug)]
struct ChunkSubmeshManager {
    chunk_index_map: KeyIndexMapper<[usize; 3]>,
    chunk_submeshes: Vec<ChunkSubmesh>,
    chunk_vertex_ranges: Vec<Range<usize>>,
    vertex_range_allocator: RangeAllocator,
    index_range_allocator: RangeAllocator,
    updated_data_ranges: Vec<ChunkSubmeshDataRanges>,
    chunks_were_removed: bool,
}

#[derive(Clone, Debug)]
struct RangeAllocator {
    free_ranges: BTreeSet<RangeByStart>,
    consecutive_ranges: Vec<Range<usize>>,
    consecutive_range_counts: Vec<usize>,
}

#[derive(Clone, Debug)]
struct RangeByStart(Range<usize>);

impl MeshedChunkedVoxelObject {
    /// Creates the [`ChunkedVoxelObjectMesh`] for the given
    /// [`ChunkedVoxelObject`] and returns them as a
    /// [`MeshedChunkedVoxelObject`].
    pub fn create(voxel_object: ChunkedVoxelObject) -> Self {
        let mesh = ChunkedVoxelObjectMesh::create(&voxel_object);
        Self {
            object: voxel_object,
            mesh,
        }
    }

    /// Returns a reference to the [`ChunkedVoxelObject`].
    pub fn object(&self) -> &ChunkedVoxelObject {
        &self.object
    }

    /// Returns a mutable reference to the [`ChunkedVoxelObject`].
    pub fn object_mut(&mut self) -> &mut ChunkedVoxelObject {
        &mut self.object
    }

    /// Returns a reference to the object's [`ChunkedVoxelObjectMesh`].
    pub fn mesh(&self) -> &ChunkedVoxelObjectMesh {
        &self.mesh
    }

    /// Recomputes the meshes for any exposed chunks in the voxel object that
    /// have been invalidated (it is assumed that this is the same voxel object
    /// used for creating the mesh initially). Invalidated mesh data may be
    /// overwritten to reuse buffer space.
    pub fn sync_mesh_with_object(&mut self) {
        self.mesh.sync_with_voxel_object(&mut self.object);
    }

    /// Signaling that the mesh modifications have been synchronized with the GPU.
    pub fn report_gpu_resources_synchronized(&mut self) {
        self.mesh.report_gpu_resources_synchronized();
    }
}

impl ChunkedVoxelObjectMesh {
    pub fn create(voxel_object: &ChunkedVoxelObject) -> Self {
        let chunk_count_heuristic = voxel_object.exposed_chunk_count_heuristic();
        let vertex_count_huristic =
            chunk_count_heuristic * Self::vertex_count_per_chunk_heuristic();
        let index_count_huristic = chunk_count_heuristic * Self::index_count_per_chunk_heuristic();

        // There is likely to be a lot of mesh data, so allocating up front tends to
        // give a significant performace gain
        let mut positions = Vec::with_capacity(vertex_count_huristic);
        let mut normal_vectors = Vec::with_capacity(vertex_count_huristic);
        let mut index_materials = Vec::with_capacity(index_count_huristic);
        let mut indices = Vec::with_capacity(index_count_huristic);

        let mut sdf_buffer = VoxelChunkSignedDistanceField::default();

        let mut surface_nets_buffer = SurfaceNetsBuffer::with_capacities(
            Self::vertex_count_per_chunk_heuristic(),
            Self::index_count_per_chunk_heuristic(),
        );

        let mut chunk_submesh_manager = ChunkSubmeshManager::with_capacity(chunk_count_heuristic);

        voxel_object.for_each_exposed_chunk_with_sdf(&mut sdf_buffer, &mut |chunk, sdf| {
            let chunk_indices = chunk.chunk_indices();

            let vertex_position_offset =
                Self::vertex_position_offset_for_chunk(voxel_object, chunk_indices);

            sdf.compute_surface_nets_mesh(
                voxel_object.voxel_extent() as f32,
                &vertex_position_offset,
                &mut surface_nets_buffer,
            );

            if surface_nets_buffer.is_empty() {
                return;
            }

            let vertex_offset = positions.len();
            let index_offset = indices.len();
            let vertex_count = surface_nets_buffer.positions.len();
            let index_count = surface_nets_buffer.indices.len();

            chunk_submesh_manager.push_chunk(
                *chunk_indices,
                vertex_offset,
                vertex_count,
                index_offset,
                index_count,
                chunk.flags(),
            );

            positions.extend_from_slice(&surface_nets_buffer.positions);
            normal_vectors.extend_from_slice(&surface_nets_buffer.normal_vectors);
            index_materials.extend_from_slice(&surface_nets_buffer.index_materials);

            indices.reserve(index_count);
            indices.extend(
                surface_nets_buffer
                    .indices
                    .iter()
                    .map(|&index| VoxelMeshIndex(vertex_offset as u32 + u32::from(index))),
            );
        });

        Self {
            positions,
            normal_vectors,
            index_materials,
            indices,
            sdf_buffer,
            surface_nets_buffer,
            chunk_submesh_manager,
        }
    }

    /// Recomputes the meshes for any exposed chunks in the voxel object that
    /// have been invalidated (it is assumed that this is the same voxel object
    /// used for creating the mesh initially). Invalidated mesh data may be
    /// overwritten to reuse buffer space.
    pub fn sync_with_voxel_object(&mut self, voxel_object: &mut ChunkedVoxelObject) {
        let invalidated_mesh_chunk_indices = voxel_object.invalidated_mesh_chunk_indices();

        for chunk_indices in invalidated_mesh_chunk_indices {
            if let Some(chunk_flags) =
                voxel_object.fill_sdf_for_chunk_if_exposed(&mut self.sdf_buffer, *chunk_indices)
            {
                let vertex_position_offset =
                    Self::vertex_position_offset_for_chunk(voxel_object, chunk_indices);

                self.sdf_buffer.compute_surface_nets_mesh(
                    voxel_object.voxel_extent() as f32,
                    &vertex_position_offset,
                    &mut self.surface_nets_buffer,
                );

                if self.surface_nets_buffer.is_empty() {
                    self.chunk_submesh_manager
                        .remove_chunk_if_present(chunk_indices);
                    continue;
                }

                let total_vertex_count = self.positions.len();
                let total_index_count = self.indices.len();
                let vertex_count = self.surface_nets_buffer.positions.len();
                let index_count = self.surface_nets_buffer.indices.len();

                let ChunkSubmeshDataRanges {
                    vertex_range,
                    index_range,
                } = self.chunk_submesh_manager.write_chunk(
                    total_vertex_count,
                    total_index_count,
                    *chunk_indices,
                    vertex_count,
                    index_count,
                    chunk_flags,
                );

                if vertex_range.start == total_vertex_count {
                    // If no free range was found for the vertex data inside the buffers, we push
                    // the data to the end of the buffers
                    self.positions
                        .extend_from_slice(&self.surface_nets_buffer.positions);
                    self.normal_vectors
                        .extend_from_slice(&self.surface_nets_buffer.normal_vectors);
                } else {
                    assert!(vertex_range.end <= total_vertex_count);
                    assert_eq!(vertex_range.len(), vertex_count);

                    // If we got a free range inside the buffers, we can use it for the new data,
                    // overwriting any obsolete values
                    self.positions[vertex_range.clone()]
                        .copy_from_slice(&self.surface_nets_buffer.positions);
                    self.normal_vectors[vertex_range.clone()]
                        .copy_from_slice(&self.surface_nets_buffer.normal_vectors);
                }

                if index_range.start == total_index_count {
                    self.index_materials
                        .extend_from_slice(&self.surface_nets_buffer.index_materials);

                    self.indices.reserve(index_count);
                    self.indices
                        .extend(self.surface_nets_buffer.indices.iter().map(
                            |&surface_nets_index| {
                                VoxelMeshIndex(
                                    vertex_range.start as u32 + u32::from(surface_nets_index),
                                )
                            },
                        ));
                } else {
                    assert!(index_range.end <= total_index_count);
                    assert_eq!(index_range.len(), index_count);

                    self.index_materials[index_range.clone()]
                        .copy_from_slice(&self.surface_nets_buffer.index_materials);

                    for (index, &surface_nets_index) in self.indices[index_range]
                        .iter_mut()
                        .zip(&self.surface_nets_buffer.indices)
                    {
                        *index = VoxelMeshIndex(
                            vertex_range.start as u32 + u32::from(surface_nets_index),
                        );
                    }
                }
            } else {
                // If the chunk is no longer exposed (most likely because it has been
                // disconnected from the object), we remove its mesh
                self.chunk_submesh_manager
                    .remove_chunk_if_present(chunk_indices);
            }
        }

        self.chunk_submesh_manager.perform_maintainance();

        voxel_object.mark_chunk_meshes_synchronized();
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
        self.chunk_submesh_manager.chunk_submeshes()
    }

    /// Returns the number of chunks in the voxel object that has associated
    /// triangles in the mesh.
    pub fn n_chunks(&self) -> usize {
        self.chunk_submeshes().len()
    }

    /// Returns the modifications that were made to the mesh since it was last
    /// synchronized with the GPU.
    pub fn mesh_modifications(&self) -> VoxelMeshModifications<'_> {
        self.chunk_submesh_manager.modifications()
    }

    /// Signaling that the mesh modifications from [`Self::mesh_modifications`]
    /// have been synchronized with the GPU.
    pub fn report_gpu_resources_synchronized(&mut self) {
        self.chunk_submesh_manager
            .report_gpu_resources_synchronized();
    }

    /// Returns a guess for the typical number of vertices in a chunk mesh.
    const fn vertex_count_per_chunk_heuristic() -> usize {
        // Most chunks contain a relatively flat surface, which would have approximately
        // the same number of vertices as the number of voxels in one chunk face
        ChunkedVoxelObject::chunk_size().pow(2)
    }

    /// Returns a guess for the typical number of indices in a chunk mesh.
    const fn index_count_per_chunk_heuristic() -> usize {
        // The surface nets algorithm tends to produce around five indices per vertex
        5 * Self::vertex_count_per_chunk_heuristic()
    }

    fn vertex_position_offset_for_chunk(
        voxel_object: &ChunkedVoxelObject,
        chunk_indices: &[usize; 3],
    ) -> Vec3A {
        let voxel_extent = voxel_object.voxel_extent() as f32;
        let chunk_extent = voxel_object.chunk_extent() as f32;

        // Since the `VoxelChunkSignedDistanceField` has a 1-voxel padding
        // around the chunk boundary, we need to subtract the voxel extent
        // from the position of the chunk's lower corner to get the offset
        // of the vertices for the surface nets mesh. We also need to add
        // half a voxel extent to account for the SDF values being specified
        // at voxel centers, at half-voxel coordinates in the voxel object.
        Vec3A::new(
            chunk_indices[0] as f32 * chunk_extent - 0.5 * voxel_extent,
            chunk_indices[1] as f32 * chunk_extent - 0.5 * voxel_extent,
            chunk_indices[2] as f32 * chunk_extent - 0.5 * voxel_extent,
        )
    }
}

impl ChunkSubmesh {
    /// Creates a new [`ChunkSubmesh`] associating the chunk at the given
    /// indices in the voxel object's chunk grid with the given index range in
    /// the index buffer of the [`ChunkedVoxelObjectMesh`].
    fn new(
        chunk_indices: [usize; 3],
        index_offset: usize,
        index_count: usize,
        flags: VoxelChunkFlags,
    ) -> Self {
        let chunk_indices = chunk_indices.map(|index| index as u32);
        let index_offset = u32::try_from(index_offset).unwrap();
        let index_count = u32::try_from(index_count).unwrap();
        let is_obscured_from_direction = Self::compute_directional_obscuredness_table(flags);

        Self {
            chunk_indices,
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

    fn index_range(&self) -> Range<usize> {
        (self.index_offset as usize)..(self.index_offset as usize + self.index_count as usize)
    }
}

impl CullingFrustum {
    /// Gathers the given frustum planes and apex position into a
    /// `CullingFrustum`.
    pub fn from_planes_and_apex_position(
        planes: [Plane<f32>; 6],
        apex_position: Point3<f32>,
    ) -> Self {
        let largest_signed_dist_aab_corner_indices_for_planes = planes.map(|plane| {
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
        frustum: &Frustum<f32>,
        transformation: &Similarity3<f32>,
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
        orthographic_frustum: &OrientedBox<f32>,
        transformation: &Similarity3<f32>,
        apex_distance: f32,
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

impl ChunkSubmeshManager {
    fn with_capacity(chunk_count: usize) -> Self {
        Self {
            chunk_index_map: KeyIndexMapper::with_capacity(chunk_count),
            chunk_submeshes: Vec::with_capacity(chunk_count),
            chunk_vertex_ranges: Vec::with_capacity(chunk_count),
            vertex_range_allocator: RangeAllocator::new(),
            index_range_allocator: RangeAllocator::new(),
            updated_data_ranges: Vec::new(),
            chunks_were_removed: false,
        }
    }

    fn chunk_submeshes(&self) -> &[ChunkSubmesh] {
        &self.chunk_submeshes
    }

    fn push_chunk(
        &mut self,
        chunk_indices: [usize; 3],
        vertex_offset: usize,
        vertex_count: usize,
        index_offset: usize,
        index_count: usize,
        flags: VoxelChunkFlags,
    ) {
        self.chunk_index_map.push_key(chunk_indices);

        self.chunk_submeshes.push(ChunkSubmesh::new(
            chunk_indices,
            index_offset,
            index_count,
            flags,
        ));

        self.chunk_vertex_ranges
            .push(vertex_offset..vertex_offset + vertex_count);
    }

    fn write_chunk(
        &mut self,
        total_vertex_count: usize,
        total_index_count: usize,
        chunk_indices: [usize; 3],
        vertex_count: usize,
        index_count: usize,
        flags: VoxelChunkFlags,
    ) -> ChunkSubmeshDataRanges {
        // If the chunk already exists, free its mesh vertex and index ranges
        let chunk_idx = self.chunk_index_map.get(chunk_indices).inspect(|&idx| {
            let old_chunk_submesh = &self.chunk_submeshes[idx];
            let old_vertex_range = &self.chunk_vertex_ranges[idx];

            self.vertex_range_allocator.free_range(old_vertex_range);
            self.index_range_allocator
                .free_range(&old_chunk_submesh.index_range());
        });

        // Find the smallest free vertex range fitting the vertex count, or append the
        // vertices at the end if no suitable range was found
        let new_vertex_range = self
            .vertex_range_allocator
            .allocate_range(vertex_count)
            .unwrap_or_else(|| total_vertex_count..total_vertex_count + vertex_count);

        // Do the same for indices
        let new_index_range = self
            .index_range_allocator
            .allocate_range(index_count)
            .unwrap_or_else(|| total_index_count..total_index_count + index_count);

        // Now that we know where the indices will begin, we can construct the
        // `ChunkSubmesh`
        let new_chunk_submesh =
            ChunkSubmesh::new(chunk_indices, new_index_range.start, index_count, flags);

        // Overwrite the existing submesh entry or push a new one if the chunk didn't
        // exist
        if let Some(idx) = chunk_idx {
            self.chunk_submeshes[idx] = new_chunk_submesh;
            self.chunk_vertex_ranges[idx] = new_vertex_range.clone();
        } else {
            self.chunk_index_map.push_key(chunk_indices);
            self.chunk_submeshes.push(new_chunk_submesh);
            self.chunk_vertex_ranges.push(new_vertex_range.clone());
        }

        let data_ranges = ChunkSubmeshDataRanges {
            vertex_range: new_vertex_range,
            index_range: new_index_range,
        };

        // Append the ranges of the new data to the staging buffer, which will be used
        // to synchronize the new data with the GPU
        self.updated_data_ranges.push(data_ranges.clone());

        data_ranges
    }

    fn remove_chunk_if_present(&mut self, chunk_indices: &[usize; 3]) {
        if let Ok(idx) = self.chunk_index_map.try_swap_remove_key(*chunk_indices) {
            let vertex_range = self.chunk_vertex_ranges.swap_remove(idx);

            let chunk_submesh = self.chunk_submeshes.swap_remove(idx);

            self.vertex_range_allocator.free_range(&vertex_range);
            self.index_range_allocator
                .free_range(&chunk_submesh.index_range());

            self.chunks_were_removed = true;
        }
    }

    fn perform_maintainance(&mut self) {
        self.vertex_range_allocator.merge_consecutive_ranges();
        self.index_range_allocator.merge_consecutive_ranges();
    }

    fn modifications(&self) -> VoxelMeshModifications<'_> {
        VoxelMeshModifications {
            updated_chunk_submesh_data_ranges: &self.updated_data_ranges,
            chunks_were_removed: self.chunks_were_removed,
        }
    }

    fn report_gpu_resources_synchronized(&mut self) {
        self.updated_data_ranges.clear();
        self.chunks_were_removed = false;
    }
}

impl RangeAllocator {
    fn new() -> Self {
        Self {
            free_ranges: BTreeSet::new(),
            // These are only needed inside `merge_consecutive_ranges`, but we store them in the
            // object so that we don't have to re-allocate them for every call
            consecutive_ranges: Vec::new(),
            consecutive_range_counts: Vec::new(),
        }
    }

    fn allocate_range(&mut self, required_len: usize) -> Option<Range<usize>> {
        assert!(required_len > 0);

        // Search through the free ranges for the smallest range that can fit the
        // required length
        let mut taken_range = None;
        let mut best_len = usize::MAX;
        for range in &self.free_ranges {
            let len = range.0.len();
            if len < best_len && len >= required_len {
                taken_range = Some(range.clone());
                best_len = len;
            }
        }

        taken_range.map(|range| {
            // If we found a range, we remove it from the list and then re-insert the part
            // of the range that we do not need
            self.free_ranges.remove(&range);

            let remaining_range = &((range.0.start + required_len)..range.0.end);
            if !remaining_range.is_empty() {
                self.free_ranges.insert(remaining_range.into());
            }

            range.0.start..range.0.start + required_len
        })
    }

    fn free_range(&mut self, range: &Range<usize>) {
        assert!(!range.is_empty());
        self.free_ranges.insert(range.into());
    }

    fn merge_consecutive_ranges(&mut self) {
        if self.free_ranges.len() < 2 {
            return;
        }

        self.consecutive_ranges.clear();
        self.consecutive_range_counts.clear();

        let mut iter = self.free_ranges.iter();
        let mut prev = iter.next().unwrap();
        for curr in iter {
            if curr.0.start == prev.0.end {
                if matches!(self.consecutive_ranges.last(), Some(last) if last.end == prev.0.end) {
                    *self.consecutive_range_counts.last_mut().unwrap() += 1;
                } else {
                    self.consecutive_ranges.push(prev.0.clone());
                    self.consecutive_range_counts.push(2);
                }
                self.consecutive_ranges.push(curr.0.clone());
            }
            prev = curr;
        }

        let mut offset = 0;
        for &count in &self.consecutive_range_counts {
            let ranges = &self.consecutive_ranges[offset..offset + count];
            for range in ranges {
                self.free_ranges.remove(&range.into());
            }
            let merged_range = &(ranges.first().unwrap().start..ranges.last().unwrap().end);
            self.free_ranges.insert(merged_range.into());
            offset += count;
        }
    }

    #[cfg(test)]
    fn verify(&self) {
        if self.free_ranges.len() < 2 {
            return;
        }
        let mut iter = self.free_ranges.iter();
        let mut prev = iter.next().unwrap();
        for curr in iter {
            assert!(curr.0.start >= prev.0.end, "Found overlapping free ranges");
            prev = curr;
        }
    }
}

impl From<&Range<usize>> for RangeByStart {
    fn from(range: &Range<usize>) -> Self {
        Self(range.clone())
    }
}

impl PartialEq for RangeByStart {
    fn eq(&self, other: &Self) -> bool {
        self.0.start == other.0.start
    }
}

impl Eq for RangeByStart {}

impl Ord for RangeByStart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.start.cmp(&other.0.start)
    }
}

impl PartialOrd for RangeByStart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_allocator_allocates_nothing_before_freed() {
        let mut alloc = RangeAllocator::new();
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn range_allocator_frees_and_allocates_single_range() {
        let mut alloc = RangeAllocator::new();
        alloc.free_range(&(2..6));
        alloc.verify();
        assert_eq!(alloc.allocate_range(4).unwrap(), 2..6);
        alloc.verify();
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn range_allocator_allocates_range_in_smallest_slot() {
        let mut alloc = RangeAllocator::new();
        alloc.free_range(&(2..6));
        alloc.free_range(&(10..12));
        alloc.verify();
        assert_eq!(alloc.allocate_range(2).unwrap(), 10..12);
        assert_eq!(alloc.allocate_range(4).unwrap(), 2..6);
        alloc.verify();
    }

    #[test]
    fn range_allocator_uses_parts_of_larger_slots() {
        let mut alloc = RangeAllocator::new();
        alloc.free_range(&(2..12));
        assert_eq!(alloc.allocate_range(4).unwrap(), 2..6);
        alloc.verify();
        assert_eq!(alloc.allocate_range(4).unwrap(), 6..10);
        alloc.verify();
        assert!(alloc.allocate_range(4).is_none());
        alloc.verify();
        assert_eq!(alloc.allocate_range(2).unwrap(), 10..12);
        alloc.verify();
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn range_allocator_does_not_merge_two_disconnected_free_ranges() {
        let mut alloc = RangeAllocator::new();
        alloc.free_range(&(2..5));
        alloc.free_range(&(6..9));
        alloc.verify();
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert!(alloc.allocate_range(6).is_none());
        alloc.verify();
    }

    #[test]
    fn range_allocator_merges_two_consecutive_free_ranges() {
        let mut alloc = RangeAllocator::new();
        alloc.free_range(&(2..6));
        alloc.free_range(&(6..8));
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert_eq!(alloc.allocate_range(6).unwrap(), 2..8);
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn range_allocator_merges_three_consecutive_free_ranges() {
        let mut alloc = RangeAllocator::new();
        alloc.free_range(&(2..6));
        alloc.free_range(&(6..8));
        alloc.free_range(&(8..42));
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert_eq!(alloc.allocate_range(40).unwrap(), 2..42);
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn range_allocator_merges_four_consecutive_free_ranges() {
        let mut alloc = RangeAllocator::new();
        alloc.free_range(&(2..6));
        alloc.free_range(&(6..8));
        alloc.free_range(&(8..42));
        alloc.free_range(&(42..50));
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert_eq!(alloc.allocate_range(48).unwrap(), 2..50);
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }
}
