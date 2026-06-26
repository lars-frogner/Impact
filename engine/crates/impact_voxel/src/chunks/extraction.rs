//! Extraction of regions of voxel objects.

use crate::{
    Voxel, VoxelFlags, VoxelSignedDistance,
    chunks::{
        CHUNK_SIZE, CHUNK_VOXEL_COUNT, ChunkedVoxelObject, FaceEmptyCounts, FaceVoxelDistribution,
        NON_EMPTY_VOXEL_THRESHOLD, NonUniformVoxelChunk, UniformVoxelChunk, VoxelChunk,
        VoxelChunkFlags, chunk_range_encompassing_voxel_range, chunk_voxels, chunk_voxels_mut,
        determine_occupied_voxel_ranges, linear_voxel_idx_within_chunk,
        split_detection::{
            CHUNK_MAX_REGIONS, GlobalRegionLabel, NonUniformChunkSplitDetectionData, SplitDetector,
            UniformChunkSplitDetectionData, chunk_voxel_region_labels, find_root_for_region,
            non_uniform_chunk_regions, non_uniform_chunk_start_region_idx,
        },
    },
    utils::{Dimension, Faces},
};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_containers::HashSet;
use impact_geometry::{AxisAlignedBox, Plane, PlaneC};
use impact_math::{
    point::Point3C,
    vector::{Vector3, Vector3C},
};
use std::{array, cmp::Ordering, mem, ops::Range};

/// Represents a helper for keeping track of the transferral of some aggregate
/// voxel property when a voxel object is extracted from another.
pub trait PropertyTransferrer {
    fn transfer_voxel(&mut self, object_voxel_indices: &[usize; 3], voxel: Voxel);

    fn transfer_non_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxels: &[Voxel]);

    fn transfer_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxel: Voxel);
}

/// Represents a helper for incrementally computing some aggregate voxel
/// property when a voxel object is extracted from another.
pub trait PropertyComputer {
    fn compute_for_voxel(&mut self, object_voxel_indices: &[usize; 3], voxel: Voxel);

    fn compute_for_non_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxels: &[Voxel]);

    fn compute_for_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxel: Voxel);
}

/// A [`ChunkedVoxelObject`] that has been extracted from a larger object.
#[derive(Clone, Debug)]
pub struct ExtractedVoxelObject {
    /// The extracted object.
    pub voxel_object: ChunkedVoxelObject,
    /// The offset in whole voxels from the origin of the parent object to the
    /// origin of the extracted object, in the reference frame of the
    /// parent object (the extracted object has the same orientation as the
    /// parent object, only the offset is different).
    pub origin_offset_in_parent: [usize; 3],
}

struct NoPropertyTransferrer;

struct NoPropertyComputer;

impl ChunkedVoxelObject {
    /// Checks if the object consists of more than one disconnected region, and
    /// if so, extracts one of them into a seperate object and returns it. Both
    /// this object and the returned object will have the correct derived state
    /// when this call returns.
    pub fn extract_any_disconnected_region(&mut self) -> Option<ExtractedVoxelObject> {
        self.extract_any_disconnected_region_with_property_transferrer(&mut NoPropertyTransferrer)
    }

    /// Checks if the object consists of more than one disconnected region, and
    /// if so, extracts one of them into a seperate object and returns it. Both
    /// this object and the returned object will have the correct derived state
    /// when this call returns. The methods of the given `PropertyTransferrer`
    /// will be called appropriately when voxels or whole chunks are copied over
    /// to the disconnected object.
    pub fn extract_any_disconnected_region_with_property_transferrer(
        &mut self,
        property_transferrer: &mut impl PropertyTransferrer,
    ) -> Option<ExtractedVoxelObject> {
        // If we just look for any disconnected region and extract it, that
        // region could turn out to contain most of the object. To avoid this,
        // we consider two regions in tandem and extract whichever of them
        // contains the fewest chunks.

        let disconnected_regions = self.find_two_disconnected_regions()?;

        let mut region_linear_chunk_indices = [Vec::with_capacity(16), Vec::with_capacity(16)];
        let mut region_non_uniform_chunk_counts = [0; 2];

        let mut min_region_chunk_indices = [[usize::MAX; 3]; 2];
        let mut max_region_chunk_indices = [[0; 3]; 2];

        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_j in self.occupied_chunk_ranges[1].clone() {
                for chunk_k in self.occupied_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    match &self.chunks[chunk_idx] {
                        VoxelChunk::NonUniform(chunk) => {
                            let chunk_data = &chunk.split_detection;

                            let start_region_idx = non_uniform_chunk_start_region_idx(
                                self.split_detector.original_uniform_chunk_count,
                                chunk.data_offset,
                            );

                            let mut found_region = [false; 2];

                            'regions_for_chunk: for region_idx in
                                0..chunk_data.region_count as usize
                            {
                                let parent_label = self.split_detector.regions
                                    [start_region_idx + region_idx]
                                    .parent_label;

                                let region_root_label = if parent_label
                                    == GlobalRegionLabel::new(chunk_idx as u32, region_idx as u32)
                                {
                                    parent_label
                                } else {
                                    find_root_for_region(
                                        &self.chunks,
                                        &self.split_detector.regions,
                                        self.split_detector.original_uniform_chunk_count,
                                        parent_label,
                                    )
                                };

                                if region_root_label == disconnected_regions[0]
                                    || region_root_label == disconnected_regions[1]
                                {
                                    // Make sure all local regions pointing to
                                    // either of our disconnected regions store
                                    // their root label directly, so that we
                                    // don't have to call `find_root_for_region`
                                    // again in `Self::split_off_region`
                                    self.split_detector.regions[start_region_idx + region_idx]
                                        .parent_label = region_root_label;
                                }

                                let idx = if region_root_label == disconnected_regions[0]
                                    && !found_region[0]
                                {
                                    0
                                } else if region_root_label == disconnected_regions[1]
                                    && !found_region[1]
                                {
                                    1
                                } else {
                                    continue 'regions_for_chunk;
                                };

                                region_linear_chunk_indices[idx].push(chunk_idx);
                                region_non_uniform_chunk_counts[idx] += 1;

                                min_region_chunk_indices[idx] = super::componentwise_min_indices(
                                    &min_region_chunk_indices[idx],
                                    &chunk_indices,
                                );
                                max_region_chunk_indices[idx] = super::componentwise_max_indices(
                                    &max_region_chunk_indices[idx],
                                    &chunk_indices,
                                );

                                found_region[idx] = true;
                            }
                        }
                        VoxelChunk::Uniform(chunk) => {
                            let parent_label = self.split_detector.regions
                                [chunk.split_detection.data_offset as usize]
                                .parent_label;
                            let region_root_label =
                                if parent_label == GlobalRegionLabel::new(chunk_idx as u32, 0) {
                                    parent_label
                                } else {
                                    find_root_for_region(
                                        &self.chunks,
                                        &self.split_detector.regions,
                                        self.split_detector.original_uniform_chunk_count,
                                        parent_label,
                                    )
                                };

                            let idx = if region_root_label == disconnected_regions[0] {
                                0
                            } else if region_root_label == disconnected_regions[1] {
                                1
                            } else {
                                continue;
                            };

                            region_linear_chunk_indices[idx].push(chunk_idx);

                            min_region_chunk_indices[idx] = super::componentwise_min_indices(
                                &min_region_chunk_indices[idx],
                                &chunk_indices,
                            );
                            max_region_chunk_indices[idx] = super::componentwise_max_indices(
                                &max_region_chunk_indices[idx],
                                &chunk_indices,
                            );
                        }
                        VoxelChunk::Void => {}
                    }
                }
            }
        }

        // We prioritize having the smallest number of non-uniform chunks in the
        // object that we extract
        let smallest_region_idx =
            match region_non_uniform_chunk_counts[0].cmp(&region_non_uniform_chunk_counts[1]) {
                Ordering::Less => 0,
                Ordering::Greater => 1,
                Ordering::Equal => {
                    // Use total non-void chunk counts to break tie
                    match region_linear_chunk_indices[0]
                        .len()
                        .cmp(&region_linear_chunk_indices[1].len())
                    {
                        Ordering::Less => 0,
                        _ => 1,
                    }
                }
            };

        let smallest_region = disconnected_regions[smallest_region_idx];

        let smallest_region_linear_chunk_indices =
            mem::take(&mut region_linear_chunk_indices[smallest_region_idx]);

        let smallest_non_uniform_chunk_count = region_non_uniform_chunk_counts[smallest_region_idx];

        assert!(!smallest_region_linear_chunk_indices.is_empty());

        let smallest_region_chunk_ranges = array::from_fn(|dim| {
            min_region_chunk_indices[smallest_region_idx][dim]
                ..max_region_chunk_indices[smallest_region_idx][dim] + 1
        });

        self.extract_disconnected_region(
            smallest_region,
            smallest_region_linear_chunk_indices,
            smallest_non_uniform_chunk_count,
            smallest_region_chunk_ranges,
            property_transferrer,
        )
    }

    fn extract_disconnected_region(
        &mut self,
        region_to_extract: GlobalRegionLabel,
        region_linear_chunk_indices: Vec<usize>,
        region_non_uniform_chunk_count: usize,
        region_chunk_ranges: [Range<usize>; 3],
        property_transferrer: &mut impl PropertyTransferrer,
    ) -> Option<ExtractedVoxelObject> {
        let region_chunk_counts = region_chunk_ranges.clone().map(|range| range.len());
        let total_region_chunk_count = region_chunk_counts.iter().product();
        let region_uniform_chunk_count =
            region_linear_chunk_indices.len() - region_non_uniform_chunk_count;

        let mut region_voxels =
            Vec::with_capacity(region_non_uniform_chunk_count * CHUNK_VOXEL_COUNT);

        let mut region_chunks = Vec::with_capacity(total_region_chunk_count);

        let mut region_split_detector =
            SplitDetector::new(region_uniform_chunk_count, region_non_uniform_chunk_count);

        // We use this to lookup if a `LocalRegionLabel` for a voxel corresponds
        // to the global region that we are extracting
        let mut split_off_voxel_at_label = [false; CHUNK_MAX_REGIONS];

        // This keeps track of how far we have gotten in
        // `region_linear_chunk_indices`
        let mut cursor = 0;
        let mut region_uniform_chunk_data_offset = 0;
        let mut region_non_uniform_chunk_data_offset = 0;

        for chunk_i in region_chunk_ranges[0].clone() {
            for chunk_j in region_chunk_ranges[1].clone() {
                for chunk_k in region_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    if region_linear_chunk_indices
                        .get(cursor)
                        .is_some_and(|&region_chunk_idx| chunk_idx == region_chunk_idx)
                    {
                        match self.chunks[chunk_idx] {
                            VoxelChunk::NonUniform(chunk) => {
                                let chunk_regions = non_uniform_chunk_regions(
                                    &self.split_detector.regions,
                                    self.split_detector.original_uniform_chunk_count,
                                    chunk.data_offset,
                                );

                                // Fill the lookup table and check if the chunk contains other
                                // regions than the one we want to extract
                                let mut is_mixed = false;
                                for region_idx in 0..chunk.split_detection.region_count as usize {
                                    // We made sure to put the root as the parent in
                                    // `Self::extract_any_disconnected_region`
                                    let region_root_label = chunk_regions[region_idx].parent_label;

                                    split_off_voxel_at_label[region_idx] =
                                        region_root_label == region_to_extract;

                                    is_mixed = is_mixed || region_root_label != region_to_extract;
                                }

                                let chunk_voxels =
                                    chunk_voxels_mut(&mut self.voxels, chunk.data_offset);

                                let mut region_chunk = NonUniformVoxelChunk {
                                    data_offset: region_non_uniform_chunk_data_offset,
                                    // We will compute the correct face distributions after moving
                                    // over the voxels
                                    face_distributions: [[FaceVoxelDistribution::Empty; 2]; 3],
                                    flags: VoxelChunkFlags::empty(),
                                    split_detection: NonUniformChunkSplitDetectionData::new(),
                                };

                                if is_mixed {
                                    // If the chunk contains voxels belonging to other regions, we
                                    // must move over the individual voxels based on their region

                                    let labels = chunk_voxel_region_labels(
                                        &self.split_detector.voxel_region_labels,
                                        chunk.data_offset,
                                    );

                                    let voxel_object_index_ranges = chunk_indices
                                        .map(|index| index * CHUNK_SIZE..(index + 1) * CHUNK_SIZE);

                                    let mut voxel_idx = 0;

                                    for i in voxel_object_index_ranges[0].clone() {
                                        for j in voxel_object_index_ranges[1].clone() {
                                            for k in voxel_object_index_ranges[2].clone() {
                                                let voxel = &mut chunk_voxels[voxel_idx];

                                                let region_voxel = if voxel.is_empty() {
                                                    // Since the signed distances of empty voxels
                                                    // adjacent to non-empty ones affect meshing, we
                                                    // copy over empty voxels unconditionally
                                                    *voxel
                                                } else if split_off_voxel_at_label
                                                    [labels[voxel_idx] as usize]
                                                {
                                                    // The voxel belongs to the region we are
                                                    // extracting, so we grab it and replace it
                                                    // with an empty voxel in the original object
                                                    let region_voxel = *voxel;

                                                    property_transferrer
                                                        .transfer_voxel(&[i, j, k], region_voxel);

                                                    *voxel = Voxel::maximally_outside();

                                                    region_voxel
                                                } else {
                                                    // The voxel belongs to some other region, so we
                                                    // write an empty voxel to the extracted object
                                                    Voxel::maximally_outside()
                                                };
                                                region_voxels.push(region_voxel);

                                                voxel_idx += 1;
                                            }
                                        }
                                    }

                                    // We have filled this chunk of the extracted object, so we
                                    // go ahead and compute the face distributions and internal
                                    // adjacencies for the chunk
                                    region_chunk
                                        .update_all_internal_state_and_determine_sparseness(
                                            &mut region_voxels,
                                        );

                                    region_split_detector.update_local_connected_regions_for_chunk(
                                        &region_voxels,
                                        &mut region_chunk,
                                        region_chunks.len() as u32,
                                    );
                                } else {
                                    property_transferrer
                                        .transfer_non_uniform_chunk(&chunk_indices, chunk_voxels);

                                    // If the chunk only contains voxels belonging to the region we
                                    // are extracting, we can copy over all the voxels in one go
                                    region_voxels.extend_from_slice(chunk_voxels);

                                    // We replace them with empty voxels and mark the original chunk
                                    // as void (although the voxels now lose their owner, we might
                                    // still encounter them when looping over all voxels for things
                                    // like aggregations, so it is still important that we make them
                                    // empty)
                                    chunk_voxels.fill(Voxel::maximally_outside());
                                    self.chunks[chunk_idx] = VoxelChunk::Void;

                                    // Since the chunk has just changed owner, the face
                                    // distributions are still valid
                                    region_chunk.face_distributions = chunk.face_distributions;

                                    // The internal connected region information can also be copied
                                    // over
                                    region_split_detector
                                        .copy_local_connected_regions_from_chunk_in_other(
                                            &mut region_chunk,
                                            region_chunks.len() as u32,
                                            &self.split_detector,
                                            &chunk,
                                        );
                                }

                                region_chunks.push(VoxelChunk::NonUniform(region_chunk));
                                region_non_uniform_chunk_data_offset += 1;

                                if let VoxelChunk::NonUniform(chunk) = &mut self.chunks[chunk_idx] {
                                    // If the original chunk still contains data, its face
                                    // distributions, internal adjacencies and connected regions
                                    // data are invalidated, so we recompute it all
                                    chunk.update_all_internal_state_and_determine_sparseness(
                                        &mut self.voxels,
                                    );

                                    // This would be sketchy if we used `find_root_for_region` for
                                    // any subsequent chunks, but by storing the root labels
                                    // directly in the `parent_label` fields of the local regions we
                                    // have avoided the need for that
                                    self.split_detector
                                        .update_local_connected_regions_for_chunk(
                                            &self.voxels,
                                            chunk,
                                            chunk_idx as u32,
                                        );
                                }

                                // The original chunk's mesh will also have to be updated. In
                                // general, the meshes of adjacent chunks are also affected when the
                                // voxels near the boundary of a chunk change. However, in this case
                                // we know that adjacent chunks that are not part of the region we
                                // are extracting will be separated from the modified voxels by
                                // empty space, so their meshes should not be affected. The adjacent
                                // chunks that are a part of the region we are splitting off will be
                                // or have already been visited in this loop, so we don't have to
                                // address them here.
                                self.invalidated_mesh_chunk_indices.insert(chunk_indices);
                            }
                            VoxelChunk::Uniform(mut chunk) => {
                                // If the chunk to extract is uniform, we can just move it over
                                // and replace it with a void chunk, after making sure to
                                // overwrite the data offset with the correct value for the
                                // extracted object

                                property_transferrer
                                    .transfer_uniform_chunk(&chunk_indices, chunk.voxel);

                                chunk.split_detection.data_offset =
                                    region_uniform_chunk_data_offset;
                                region_uniform_chunk_data_offset += 1;

                                region_chunks.push(VoxelChunk::Uniform(chunk));
                                self.chunks[chunk_idx] = VoxelChunk::Void;
                            }
                            VoxelChunk::Void => {
                                unreachable!()
                            }
                        }
                        cursor += 1;
                    } else {
                        // We need to pad with void chunks to fill up the whole chunk grid of the
                        // extracted object
                        region_chunks.push(VoxelChunk::Void);
                    }
                }
            }
        }

        // Now that we have removed part of the original object, we may be able
        // to shrink the recorded occupied chunk and voxel ranges
        self.update_occupied_ranges();

        // Any chunk within or adjacent to the block of chunks covering the
        // extracted region may have invalidated adjacencies
        self.update_upper_boundary_adjacencies_for_chunks_in_ranges(
            region_chunk_ranges
                .clone()
                .map(|range| range.start.saturating_sub(1)..range.end),
        );

        // We also have to resolve the globally connected regions now that the
        // original object has been modified
        self.resolve_connected_regions_between_all_chunks();

        let mut extracted = Self::complete_extracted_voxel_object(
            self.voxel_extent,
            &self.origin_offset_in_root,
            region_chunk_counts,
            region_chunk_ranges,
            region_uniform_chunk_count,
            region_non_uniform_chunk_count,
            region_voxels,
            region_chunks,
            region_split_detector,
        )?;

        // We have already computed the internal adjacencies and local region
        // connectivity in the extracted object, but all derived state between
        // chunks must be computed from scratch

        extracted
            .voxel_object
            .update_all_chunk_boundary_adjacencies();

        extracted
            .voxel_object
            .resolve_connected_regions_between_all_chunks();

        // Also make sure to tighten the voxel ranges
        extracted.voxel_object.update_occupied_ranges();

        Some(extracted)
    }

    /// The AABB and planes should be specified in the normalized model space of
    /// the voxel object, where distances are in voxels, the lower corner of the
    /// grid is at the origin and the cartesian axes are aligned with the grid.
    pub fn extract_polyhedron(
        &mut self,
        normalized_aabb: &AxisAlignedBox,
        normalized_face_planes: &[PlaneC],
    ) -> Option<ExtractedVoxelObject> {
        self.extract_polyhedron_with_property_transferrer(
            normalized_aabb,
            normalized_face_planes,
            &mut NoPropertyTransferrer,
        )
    }

    /// The AABB and planes should be specified in the normalized model space of
    /// the voxel object, where distances are in voxels, the lower corner of the
    /// grid is at the origin and the cartesian axes are aligned with the grid.
    pub fn extract_polyhedron_with_property_transferrer(
        &mut self,
        normalized_aabb: &AxisAlignedBox,
        normalized_face_planes: &[PlaneC],
        property_transferrer: &mut impl PropertyTransferrer,
    ) -> Option<ExtractedVoxelObject> {
        const EXTERIOR_MARGIN: f32 = VoxelSignedDistance::MAX_F32;
        const INTERIOR_MARGIN: f32 = -VoxelSignedDistance::MIN_F32;

        let expanded_aabb = normalized_aabb.expanded_about_center(EXTERIOR_MARGIN);

        let poly_voxel_ranges = self.voxel_ranges_in_object_touching_aab(&expanded_aabb);

        if poly_voxel_ranges.iter().any(Range::is_empty) {
            return None;
        }

        let poly_chunk_ranges = poly_voxel_ranges
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        let poly_chunk_counts = poly_chunk_ranges.clone().map(|range| range.len());
        let total_poly_chunk_count: usize = poly_chunk_counts.iter().product();

        let mut touched_non_uniform_chunk_count = 0;

        for chunk_i in poly_chunk_ranges[0].clone() {
            for chunk_j in poly_chunk_ranges[1].clone() {
                for chunk_k in poly_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    let chunk = &self.chunks[chunk_idx];

                    if let VoxelChunk::NonUniform(_) = chunk {
                        touched_non_uniform_chunk_count += 1;
                    }
                }
            }
        }

        let arena = ArenaPool::get_arena();

        let n_faces = normalized_face_planes.len();
        let mut inner_planes = AVec::with_capacity_in(n_faces, &arena);
        let mut outer_planes = AVec::with_capacity_in(n_faces, &arena);
        let mut intersecting_planes = AVec::with_capacity_in(n_faces, &arena);

        for plane in normalized_face_planes {
            let plane = plane.aligned();
            inner_planes.push(Plane::new(
                *plane.unit_normal(),
                plane.displacement() - INTERIOR_MARGIN,
            ));
            outer_planes.push(Plane::new(
                *plane.unit_normal(),
                plane.displacement() + EXTERIOR_MARGIN,
            ));
        }

        let mut poly_voxels =
            Vec::with_capacity(touched_non_uniform_chunk_count * CHUNK_VOXEL_COUNT);

        let mut poly_chunks = Vec::with_capacity(total_poly_chunk_count);

        let mut non_uniform_chunks_inside =
            AVec::with_capacity_in(touched_non_uniform_chunk_count, &arena);
        let mut non_uniform_chunks_intersecting =
            AVec::with_capacity_in(touched_non_uniform_chunk_count, &arena);

        let mut invalidated_upper_face_chunks = HashSet::with_capacity_and_hasher_in(
            touched_non_uniform_chunk_count,
            Default::default(),
            &arena,
        );

        let mut poly_uniform_chunk_count = 0;
        let mut poly_non_uniform_chunk_count = 0;

        for chunk_i in poly_chunk_ranges[0].clone() {
            for chunk_j in poly_chunk_ranges[1].clone() {
                for chunk_k in poly_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    let chunk = self.chunks[chunk_idx];

                    if let VoxelChunk::Void = chunk {
                        poly_chunks.push(VoxelChunk::Void);
                        continue;
                    }

                    let chunk_aabb = Self::compute_normalized_chunk_bounds(chunk_indices);

                    if outer_planes.iter().any(|outer_plane| {
                        chunk_aabb.lies_in_positive_halfspace_of_plane(outer_plane)
                    }) {
                        poly_chunks.push(VoxelChunk::Void);
                        continue;
                    }

                    intersecting_planes.clear();
                    for (plane_idx, inner_plane) in inner_planes.iter().enumerate() {
                        if !chunk_aabb.lies_in_negative_halfspace_of_plane(inner_plane) {
                            intersecting_planes.push(normalized_face_planes[plane_idx].aligned());
                        }
                    }

                    let is_fully_inside = intersecting_planes.is_empty();

                    let mut invalidated_faces = Faces::empty();

                    if is_fully_inside {
                        match chunk {
                            VoxelChunk::NonUniform(chunk) => {
                                let chunk_voxels =
                                    chunk_voxels_mut(&mut self.voxels, chunk.data_offset);

                                property_transferrer
                                    .transfer_non_uniform_chunk(&chunk_indices, chunk_voxels);

                                // If the chunk only contains voxels belonging to the region we
                                // are extracting, we can copy over all the voxels in one go
                                poly_voxels.extend_from_slice(chunk_voxels);

                                // We replace them with empty voxels and mark the original chunk
                                // as void (although the voxels now lose their owner, we might
                                // still encounter them when looping over all voxels for things
                                // like aggregations, so it is still important that we make them
                                // empty)
                                chunk_voxels.fill(Voxel::maximally_outside());
                                self.chunks[chunk_idx] = VoxelChunk::Void;

                                let face_distributions = chunk.face_distributions;

                                let poly_chunk = NonUniformVoxelChunk {
                                    data_offset: poly_non_uniform_chunk_count as u32,
                                    // Since the chunk has just changed owner, the face
                                    // distributions are still valid
                                    face_distributions,
                                    // We must retain the emptiness flag, the
                                    // others are determined later
                                    flags: chunk.flags & VoxelChunkFlags::HAS_ONLY_EMPTY_VOXELS,
                                    split_detection: NonUniformChunkSplitDetectionData::new(),
                                };

                                non_uniform_chunks_inside.push((chunk, poly_chunks.len()));

                                poly_chunks.push(VoxelChunk::NonUniform(poly_chunk));
                                poly_non_uniform_chunk_count += 1;

                                for (dim, distributions) in
                                    face_distributions.into_iter().enumerate()
                                {
                                    if !distributions[0].is_empty() {
                                        invalidated_faces |= Faces::all_lower()[dim];
                                    }
                                    if !distributions[1].is_empty() {
                                        invalidated_faces |= Faces::all_upper()[dim];
                                    }
                                }
                            }
                            VoxelChunk::Uniform(mut chunk) => {
                                // If the chunk to extract is uniform, we can
                                // just move it over and replace it with a void
                                // chunk, after making sure to overwrite the
                                // data offset with the correct value for the
                                // polyhedron object

                                property_transferrer
                                    .transfer_uniform_chunk(&chunk_indices, chunk.voxel);

                                chunk.split_detection.data_offset = poly_uniform_chunk_count as u32;
                                poly_uniform_chunk_count += 1;

                                poly_chunks.push(VoxelChunk::Uniform(chunk));
                                self.chunks[chunk_idx] = VoxelChunk::Void;

                                invalidated_faces = Faces::all();
                            }
                            VoxelChunk::Void => unreachable!(),
                        }
                    } else {
                        self.chunks[chunk_idx].convert_to_non_uniform_if_uniform(
                            &mut self.voxels,
                            &mut self.split_detector,
                        );

                        let VoxelChunk::NonUniform(chunk) = &mut self.chunks[chunk_idx] else {
                            unreachable!();
                        };

                        let chunk_voxels = chunk_voxels_mut(&mut self.voxels, chunk.data_offset);

                        let chunk_start_voxel_indices = chunk_indices.map(|idx| idx * CHUNK_SIZE);

                        let lower_voxel_pos = chunk_aabb.lower_corner() + Vector3::same(0.5);

                        let mut voxel_idx = 0;

                        for i in 0..CHUNK_SIZE {
                            let obj_i = chunk_start_voxel_indices[0] + i;

                            for j in 0..CHUNK_SIZE {
                                let obj_j = chunk_start_voxel_indices[1] + j;

                                for k in 0..CHUNK_SIZE {
                                    let obj_k = chunk_start_voxel_indices[2] + k;

                                    let voxel = &mut chunk_voxels[voxel_idx];
                                    let mut poly_voxel = *voxel;

                                    let position = lower_voxel_pos
                                        + Vector3::new(i as f32, j as f32, k as f32);

                                    let mut planes_signed_distance =
                                        intersecting_planes[0].compute_signed_distance(&position);

                                    for plane in &intersecting_planes[1..] {
                                        planes_signed_distance = planes_signed_distance
                                            .max(plane.compute_signed_distance(&position));
                                    }

                                    let planes_signed_distance =
                                        VoxelSignedDistance::from_f32(planes_signed_distance);

                                    if voxel.signed_distance.is_negative()
                                        && planes_signed_distance.is_negative()
                                    {
                                        property_transferrer
                                            .transfer_voxel(&[obj_i, obj_j, obj_k], *voxel);
                                    }

                                    voxel.signed_distance =
                                        voxel.signed_distance.max(-planes_signed_distance);

                                    poly_voxel.signed_distance =
                                        poly_voxel.signed_distance.max(planes_signed_distance);

                                    let voxel_is_non_empty = voxel.signed_distance.is_negative();
                                    let poly_voxel_is_non_empty =
                                        poly_voxel.signed_distance.is_negative();

                                    voxel.flags.set(VoxelFlags::IS_EMPTY, !voxel_is_non_empty);

                                    poly_voxel
                                        .flags
                                        .set(VoxelFlags::IS_EMPTY, !poly_voxel_is_non_empty);

                                    poly_voxels.push(poly_voxel);

                                    voxel_idx += 1;
                                }
                            }
                        }

                        invalidated_faces = Faces::all();

                        // The original chunk's face distributions, internal
                        // adjacencies and connected regions data are
                        // invalidated, so we recompute it all
                        chunk.update_all_internal_state_and_determine_sparseness(&mut self.voxels);

                        self.split_detector
                            .update_local_connected_regions_for_chunk(
                                &self.voxels,
                                chunk,
                                chunk_idx as u32,
                            );

                        let mut poly_chunk = NonUniformVoxelChunk {
                            data_offset: poly_non_uniform_chunk_count as u32,
                            ..Default::default()
                        };

                        // We have filled this chunk of the extracted object, so we
                        // go ahead and compute the face distributions and internal
                        // adjacencies for the chunk
                        poly_chunk
                            .update_all_internal_state_and_determine_sparseness(&mut poly_voxels);

                        non_uniform_chunks_intersecting.push(poly_chunks.len());

                        poly_chunks.push(VoxelChunk::NonUniform(poly_chunk));
                        poly_non_uniform_chunk_count += 1;
                    }

                    if !invalidated_faces.is_empty() {
                        for dim in Dimension::all() {
                            if invalidated_faces.contains(Faces::all_lower()[dim.idx()])
                                && chunk_indices[dim.idx()]
                                    > self.occupied_chunk_ranges[dim.idx()].start
                            {
                                let mut lower_chunk_indices = chunk_indices;
                                lower_chunk_indices[dim.idx()] -= 1;
                                let lower_chunk_idx = self.linear_chunk_idx(&lower_chunk_indices);
                                invalidated_upper_face_chunks.insert((lower_chunk_idx, dim));
                            }
                            if invalidated_faces.contains(Faces::all_upper()[dim.idx()])
                                && chunk_indices[dim.idx()]
                                    < self.occupied_chunk_ranges[dim.idx()].end - 1
                            {
                                invalidated_upper_face_chunks.insert((chunk_idx, dim));
                            }
                        }
                    }

                    // The original chunk's mesh will also have to be updated,
                    // as well as the meshes of its adjacent chunks
                    self.invalidated_mesh_chunk_indices.insert(chunk_indices);

                    for dim in 0..3 {
                        if chunk_indices[dim] > 0 {
                            let mut neighbor_chunk_indices = chunk_indices;
                            neighbor_chunk_indices[dim] -= 1;
                            self.invalidated_mesh_chunk_indices
                                .insert(neighbor_chunk_indices);
                        }
                        if chunk_indices[dim] < self.chunk_counts[dim] - 1 {
                            let mut neighbor_chunk_indices = chunk_indices;
                            neighbor_chunk_indices[dim] += 1;
                            self.invalidated_mesh_chunk_indices
                                .insert(neighbor_chunk_indices);
                        }
                    }
                }
            }
        }

        let mut poly_split_detector =
            SplitDetector::new(poly_uniform_chunk_count, poly_non_uniform_chunk_count);

        for (chunk, poly_chunk_idx) in non_uniform_chunks_inside {
            // The internal connected region information can be copied over
            // directly
            let poly_chunk = &mut poly_chunks[poly_chunk_idx];
            if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
                poly_split_detector.copy_local_connected_regions_from_chunk_in_other(
                    poly_chunk,
                    poly_chunk_idx as u32,
                    &self.split_detector,
                    &chunk,
                );
            }
        }

        for poly_chunk_idx in non_uniform_chunks_intersecting {
            let poly_chunk = &mut poly_chunks[poly_chunk_idx];
            if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
                poly_split_detector.update_local_connected_regions_for_chunk(
                    &poly_voxels,
                    poly_chunk,
                    poly_chunk_idx as u32,
                );
            }
        }

        // Now that we have removed part of the original object, we may be able
        // to shrink the recorded occupied chunk and voxel ranges
        self.update_occupied_ranges();

        self.update_upper_boundary_adjacencies_along_dim_for_chunks(invalidated_upper_face_chunks);

        // We also have to resolve the globally connected regions now that the
        // original object has been modified
        self.resolve_connected_regions_between_all_chunks();

        let mut extracted = Self::complete_extracted_voxel_object(
            self.voxel_extent,
            &self.origin_offset_in_root,
            poly_chunk_counts,
            poly_chunk_ranges,
            poly_uniform_chunk_count,
            poly_non_uniform_chunk_count,
            poly_voxels,
            poly_chunks,
            poly_split_detector,
        )?;

        // We have already computed the internal adjacencies and local region
        // connectivity in the extracted object, but all derived state between
        // chunks must be computed from scratch

        extracted
            .voxel_object
            .update_all_chunk_boundary_adjacencies();

        extracted
            .voxel_object
            .resolve_connected_regions_between_all_chunks();

        // Also make sure to tighten the voxel ranges
        extracted.voxel_object.update_occupied_ranges();

        Some(extracted)
    }

    /// Creates a new voxel object from the part of this object inside the
    /// polyhedron with the given AABB and face planes.
    ///
    /// The AABB and planes should be specified in the normalized model space of
    /// the voxel object, where distances are in voxels, the lower corner of the
    /// grid is at the origin and the cartesian axes are aligned with the grid.
    pub fn copy_polyhedron(
        &self,
        normalized_aabb: &AxisAlignedBox,
        normalized_face_planes: &[PlaneC],
    ) -> Option<ExtractedVoxelObject> {
        self.copy_polyhedron_with_property_computer(
            normalized_aabb,
            normalized_face_planes,
            &mut NoPropertyComputer,
        )
    }

    /// Creates a new voxel object from the part of this object inside the
    /// polyhedron with the given AABB and face planes.
    ///
    /// The AABB and planes should be specified in the normalized model space of
    /// the voxel object, where distances are in voxels, the lower corner of the
    /// grid is at the origin and the cartesian axes are aligned with the grid.
    ///
    /// The methods of the given `PropertyComputer` will be called appropriately
    /// when voxels or whole chunks are copied over to the new object.
    pub fn copy_polyhedron_with_property_computer(
        &self,
        normalized_aabb: &AxisAlignedBox,
        normalized_face_planes: &[PlaneC],
        property_computer: &mut impl PropertyComputer,
    ) -> Option<ExtractedVoxelObject> {
        #![allow(clippy::needless_range_loop)]

        // Outside the exterior margin of the polyhedron, all voxels will be
        // void
        const EXTERIOR_MARGIN: f32 = VoxelSignedDistance::MAX_F32;

        // Inside the interior margin, all voxels will be identical to their
        // counterpart in the original object
        const INTERIOR_MARGIN: f32 = -VoxelSignedDistance::MIN_F32;

        // All voxels outside the expanded AABB will be void
        let expanded_aabb = normalized_aabb.expanded_about_center(EXTERIOR_MARGIN);

        let poly_voxel_ranges = self.voxel_ranges_in_object_touching_aab(&expanded_aabb);

        if poly_voxel_ranges.iter().any(Range::is_empty) {
            return None;
        }

        let poly_chunk_ranges = poly_voxel_ranges
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        let poly_chunk_counts = poly_chunk_ranges.clone().map(|range| range.len());
        let total_poly_chunk_count: usize = poly_chunk_counts.iter().product();

        // Count tentative non-uniform chunks to find suitable capacity for
        // voxel and chunk vectors
        let mut touched_non_uniform_chunk_count = 0;

        for chunk_i in poly_chunk_ranges[0].clone() {
            for chunk_j in poly_chunk_ranges[1].clone() {
                for chunk_k in poly_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    let chunk = &self.chunks[chunk_idx];

                    if let VoxelChunk::NonUniform(_) = chunk {
                        touched_non_uniform_chunk_count += 1;
                    }
                }
            }
        }

        let arena = ArenaPool::get_arena();

        let n_faces = normalized_face_planes.len();
        let mut inner_planes = AVec::with_capacity_in(n_faces, &arena);
        let mut outer_planes = AVec::with_capacity_in(n_faces, &arena);
        let mut intersecting_planes = AVec::with_capacity_in(n_faces, &arena);

        // Create planes shifted by the interior and exterior margin
        for plane in normalized_face_planes {
            let plane = plane.aligned();
            inner_planes.push(Plane::new(
                *plane.unit_normal(),
                plane.displacement() - INTERIOR_MARGIN,
            ));
            outer_planes.push(Plane::new(
                *plane.unit_normal(),
                plane.displacement() + EXTERIOR_MARGIN,
            ));
        }

        let mut poly_voxels =
            Vec::with_capacity(touched_non_uniform_chunk_count * CHUNK_VOXEL_COUNT);

        let mut poly_chunks = Vec::with_capacity(total_poly_chunk_count);

        // Lists of chunks we must update connected regions for
        let mut non_uniform_chunks_inside =
            AVec::with_capacity_in(touched_non_uniform_chunk_count, &arena);
        let mut non_uniform_chunks_intersecting =
            AVec::with_capacity_in(touched_non_uniform_chunk_count, &arena);

        // Actual chunk counts
        let mut poly_uniform_chunk_count = 0;
        let mut poly_non_uniform_chunk_count = 0;

        for chunk_i in poly_chunk_ranges[0].clone() {
            for chunk_j in poly_chunk_ranges[1].clone() {
                for chunk_k in poly_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    let chunk = &self.chunks[chunk_idx];

                    if let VoxelChunk::Void = chunk {
                        poly_chunks.push(VoxelChunk::Void);
                        continue;
                    }

                    let chunk_aabb = Self::compute_normalized_chunk_bounds(chunk_indices);

                    // If the chunk lies outside any of the outer planes, it is
                    // outside the polyhedron and we know it must be fully void
                    if outer_planes.iter().any(|outer_plane| {
                        chunk_aabb.lies_in_positive_halfspace_of_plane(outer_plane)
                    }) {
                        poly_chunks.push(VoxelChunk::Void);
                        continue;
                    }

                    // Gather all planes for which the chunk is not inside the
                    // inner margin. The resulting planes are the ones whose
                    // margins will actually intersect the chunk.
                    intersecting_planes.clear();
                    for (plane_idx, inner_plane) in inner_planes.iter().enumerate() {
                        if !chunk_aabb.lies_in_negative_halfspace_of_plane(inner_plane) {
                            intersecting_planes.push(normalized_face_planes[plane_idx]);
                        }
                    }

                    // If there are no intersecting planes (slabs), the whole
                    // chunk is inside the inner margins of the polyhedron
                    let is_fully_inside = intersecting_planes.is_empty();

                    if is_fully_inside {
                        match chunk {
                            VoxelChunk::NonUniform(chunk) => {
                                let chunk_voxels = chunk_voxels(&self.voxels, chunk.data_offset);

                                property_computer
                                    .compute_for_non_uniform_chunk(&chunk_indices, chunk_voxels);

                                // None of the voxels are affected, so we can
                                // copy them over in one go
                                poly_voxels.extend_from_slice(chunk_voxels);

                                let poly_chunk = NonUniformVoxelChunk {
                                    data_offset: poly_non_uniform_chunk_count as u32,
                                    // The face distributions are still valid
                                    face_distributions: chunk.face_distributions,
                                    // We must retain the emptiness flag, while the
                                    // obscuredness flags are determined later
                                    flags: chunk.flags & VoxelChunkFlags::HAS_ONLY_EMPTY_VOXELS,
                                    split_detection: NonUniformChunkSplitDetectionData::new(),
                                };

                                non_uniform_chunks_inside.push((chunk, poly_chunks.len()));

                                poly_chunks.push(VoxelChunk::NonUniform(poly_chunk));
                                poly_non_uniform_chunk_count += 1;
                            }
                            VoxelChunk::Uniform(chunk) => {
                                property_computer
                                    .compute_for_uniform_chunk(&chunk_indices, chunk.voxel);

                                let poly_chunk = UniformVoxelChunk {
                                    voxel: chunk.voxel,
                                    split_detection: UniformChunkSplitDetectionData {
                                        data_offset: poly_uniform_chunk_count as u32,
                                    },
                                };

                                poly_chunks.push(VoxelChunk::Uniform(poly_chunk));
                                poly_uniform_chunk_count += 1;
                            }
                            VoxelChunk::Void => unreachable!(),
                        }
                    } else {
                        let start_voxel_idx = poly_voxels.len();

                        // We start by copying over all the chunk voxels unmodified
                        match chunk {
                            VoxelChunk::Uniform(chunk) => {
                                poly_voxels
                                    .resize(start_voxel_idx + CHUNK_VOXEL_COUNT, chunk.voxel);
                            }
                            VoxelChunk::NonUniform(chunk) => {
                                poly_voxels.extend_from_slice(chunk_voxels(
                                    &self.voxels,
                                    chunk.data_offset,
                                ));
                            }
                            VoxelChunk::Void => unreachable!(),
                        }

                        let poly_data_offset = poly_non_uniform_chunk_count as u32;

                        let poly_chunk_voxels =
                            chunk_voxels_mut(&mut poly_voxels, poly_data_offset);

                        let chunk_start_voxel_indices = chunk_indices.map(|idx| idx * CHUNK_SIZE);

                        // Position of the center of voxel in the chunk's lower corner
                        let lower_voxel_pos =
                            chunk_aabb.lower_corner().compact() + Vector3C::same(0.5);

                        // The maximum signed distance from the planes will be
                        // computed vectorized for one row at a time
                        let mut max_signed_dists_for_row =
                            [VoxelSignedDistance::maximally_inside(); CHUNK_SIZE];

                        let mut lower_occupied_voxel_indices = [CHUNK_SIZE; 3];
                        let mut upper_occupied_voxel_indices = [0; 3];

                        let mut face_empty_counts = FaceEmptyCounts::zero();
                        let mut chunk_has_only_empty_voxels = true;
                        let mut chunk_is_void = true;

                        let mut voxel_idx = 0;

                        for i in 0..CHUNK_SIZE {
                            let obj_i = i + chunk_start_voxel_indices[0];

                            let on_lower_x_face = i == 0;
                            let on_upper_x_face = i == CHUNK_SIZE - 1;

                            for j in 0..CHUNK_SIZE {
                                let obj_j = j + chunk_start_voxel_indices[1];

                                let on_lower_y_face = j == 0;
                                let on_upper_y_face = j == CHUNK_SIZE - 1;

                                Self::compute_max_plane_signed_dists_for_row(
                                    &mut max_signed_dists_for_row,
                                    &intersecting_planes,
                                    &lower_voxel_pos,
                                    i,
                                    j,
                                );

                                let mut lower_occupied_k = CHUNK_SIZE;
                                let mut upper_occupied_k = 0;

                                let mut row_empty_count = 0;

                                for k in 0..CHUNK_SIZE {
                                    let obj_k = k + chunk_start_voxel_indices[2];

                                    let poly_voxel = NonUniformVoxelChunk::get_voxel_mut(
                                        poly_chunk_voxels,
                                        voxel_idx,
                                    );

                                    let planes_signed_distance = max_signed_dists_for_row[k];

                                    // The signed distance of the polyhedron voxel is the
                                    // maximum of the original signed distance and the signed
                                    // distance from the planes
                                    poly_voxel.signed_distance =
                                        poly_voxel.signed_distance.max(planes_signed_distance);

                                    let poly_voxel_is_non_empty =
                                        poly_voxel.signed_distance.is_negative();

                                    if poly_voxel_is_non_empty {
                                        poly_voxel.flags -= VoxelFlags::IS_EMPTY;
                                        chunk_has_only_empty_voxels = false;
                                        chunk_is_void = false;

                                        lower_occupied_k = lower_occupied_k.min(k);
                                        upper_occupied_k = upper_occupied_k.max(k);

                                        property_computer
                                            .compute_for_voxel(&[obj_i, obj_j, obj_k], *poly_voxel);

                                        // Voxels inside the planes will always retain their
                                        // emptiness status. Sufficiently far inside the
                                        // planes, we know that all adjacent voxels will have
                                        // unchanged emptiness, so we can skip the adjacency
                                        // update.
                                        if !planes_signed_distance.is_maximally_inside() {
                                            let mut poly_voxel = *poly_voxel;

                                            Self::update_lower_adjacencies_for_non_empty_voxel(
                                                poly_chunk_voxels,
                                                &mut poly_voxel,
                                                [i, j, k],
                                            );

                                            *NonUniformVoxelChunk::get_voxel_mut(
                                                poly_chunk_voxels,
                                                voxel_idx,
                                            ) = poly_voxel;
                                        }
                                    } else {
                                        poly_voxel.flags |= VoxelFlags::IS_EMPTY;

                                        row_empty_count += 1;

                                        if k == 0 {
                                            face_empty_counts.increment_z_dn();
                                        } else if k == CHUNK_SIZE - 1 {
                                            face_empty_counts.increment_z_up();
                                        }

                                        if !poly_voxel.signed_distance.is_void() {
                                            chunk_is_void = false;
                                        }

                                        // Voxels outside the planes will always be
                                        // emptied. Sufficiently far outside the planes,
                                        // we know that all adjacent voxels will be
                                        // empty, so we can just clear the adjacency
                                        // flags.
                                        if planes_signed_distance.is_maximally_outside() {
                                            poly_voxel.flags &= VoxelFlags::IS_EMPTY;
                                        } else if !planes_signed_distance.is_maximally_inside() {
                                            Self::update_lower_adjacencies_for_empty_voxel(
                                                poly_chunk_voxels,
                                                [i, j, k],
                                            );
                                        }
                                    }

                                    voxel_idx += 1;
                                }

                                if lower_occupied_k <= upper_occupied_k {
                                    lower_occupied_voxel_indices[0] =
                                        lower_occupied_voxel_indices[0].min(i);
                                    upper_occupied_voxel_indices[0] =
                                        upper_occupied_voxel_indices[0].max(i);
                                    lower_occupied_voxel_indices[1] =
                                        lower_occupied_voxel_indices[1].min(j);
                                    upper_occupied_voxel_indices[1] =
                                        upper_occupied_voxel_indices[1].max(j);
                                    lower_occupied_voxel_indices[2] =
                                        lower_occupied_voxel_indices[2].min(lower_occupied_k);
                                    upper_occupied_voxel_indices[2] =
                                        upper_occupied_voxel_indices[2].max(upper_occupied_k);
                                }

                                if on_lower_x_face {
                                    face_empty_counts.add_x_dn(row_empty_count);
                                } else if on_upper_x_face {
                                    face_empty_counts.add_x_up(row_empty_count);
                                }
                                if on_lower_y_face {
                                    face_empty_counts.add_y_dn(row_empty_count);
                                } else if on_upper_y_face {
                                    face_empty_counts.add_y_up(row_empty_count);
                                }
                            }
                        }

                        if chunk_is_void {
                            poly_voxels.truncate(start_voxel_idx);
                            poly_chunks.push(VoxelChunk::Void);
                        } else {
                            let poly_chunk = NonUniformVoxelChunk {
                                data_offset: poly_data_offset,
                                face_distributions: face_empty_counts.to_chunk_face_distributions(),
                                flags: if chunk_has_only_empty_voxels {
                                    VoxelChunkFlags::HAS_ONLY_EMPTY_VOXELS
                                } else {
                                    VoxelChunkFlags::empty()
                                },
                                ..Default::default()
                            };

                            let occupied_chunk_voxel_ranges: [_; 3] = array::from_fn(|dim| {
                                lower_occupied_voxel_indices[dim]
                                    ..(upper_occupied_voxel_indices[dim] + 1).min(CHUNK_SIZE)
                            });

                            non_uniform_chunks_intersecting
                                .push((poly_chunks.len(), occupied_chunk_voxel_ranges));

                            poly_chunks.push(VoxelChunk::NonUniform(poly_chunk));
                            poly_non_uniform_chunk_count += 1;
                        }
                    }
                }
            }
        }

        let mut poly_split_detector =
            SplitDetector::new(poly_uniform_chunk_count, poly_non_uniform_chunk_count);

        for (chunk, poly_chunk_idx) in non_uniform_chunks_inside {
            // The internal connected region information can be copied over
            // directly
            let poly_chunk = &mut poly_chunks[poly_chunk_idx];
            if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
                poly_split_detector.copy_local_connected_regions_from_chunk_in_other(
                    poly_chunk,
                    poly_chunk_idx as u32,
                    &self.split_detector,
                    chunk,
                );
            }
        }

        for (poly_chunk_idx, occupied_chunk_voxel_ranges) in non_uniform_chunks_intersecting {
            let poly_chunk = &mut poly_chunks[poly_chunk_idx];
            if let VoxelChunk::NonUniform(poly_chunk) = poly_chunk {
                poly_split_detector
                    .update_local_connected_regions_within_occupied_ranges_for_chunk(
                        &poly_voxels,
                        poly_chunk,
                        poly_chunk_idx as u32,
                        &occupied_chunk_voxel_ranges,
                    );
            }
        }

        let mut extracted = Self::complete_extracted_voxel_object(
            self.voxel_extent,
            &self.origin_offset_in_root,
            poly_chunk_counts,
            poly_chunk_ranges,
            poly_uniform_chunk_count,
            poly_non_uniform_chunk_count,
            poly_voxels,
            poly_chunks,
            poly_split_detector,
        )?;

        // We have already computed the internal adjacencies and local region
        // connectivity in the extracted object, but all derived state between
        // chunks must be computed from scratch

        extracted
            .voxel_object
            .update_all_chunk_boundary_adjacencies();

        extracted
            .voxel_object
            .resolve_connected_regions_between_all_chunks();

        // Also make sure to tighten the voxel ranges
        extracted.voxel_object.update_occupied_ranges();

        Some(extracted)
    }

    #[inline]
    fn compute_max_plane_signed_dists_for_row(
        max_signed_dists_for_row: &mut [VoxelSignedDistance; CHUNK_SIZE],
        planes: &[PlaneC],
        lower_voxel_pos: &Point3C,
        i: usize,
        j: usize,
    ) {
        #![allow(clippy::needless_range_loop)]

        let row_start_pos = lower_voxel_pos + Vector3C::new(i as f32, j as f32, 0.0);

        // The following loops are designed to be easily vectorizable by the
        // compiler

        let mut max_signed_dists = [0.0; CHUNK_SIZE];

        // Initialize with the signed distances from the first plane
        let base = planes[0].compute_signed_distance(&row_start_pos);
        let step = planes[0].unit_normal().z();
        for k in 0..CHUNK_SIZE {
            max_signed_dists[k] = base + step * k as f32;
        }

        // Fold in the signed distances of the remaining planes with a max
        for plane in &planes[1..] {
            let base = plane.compute_signed_distance(&row_start_pos);
            let step = plane.unit_normal().z();
            for k in 0..CHUNK_SIZE {
                max_signed_dists[k] = max_signed_dists[k].max(base + step * k as f32);
            }
        }

        VoxelSignedDistance::from_f32_array(&max_signed_dists, max_signed_dists_for_row);
    }

    #[inline]
    fn update_lower_adjacencies_for_non_empty_voxel(
        chunk_voxels: &mut [Voxel],
        voxel: &mut Voxel,
        [i, j, k]: [usize; 3],
    ) {
        let mut update_adjacencies = |adjacent_indices, flag_for_current, flag_for_adjacent| {
            let adjacent_idx = linear_voxel_idx_within_chunk(&adjacent_indices);
            let adjacent_voxel = NonUniformVoxelChunk::get_voxel_mut(chunk_voxels, adjacent_idx);

            if adjacent_voxel.signed_distance.is_negative() {
                voxel.flags |= flag_for_current;
                adjacent_voxel.flags |= flag_for_adjacent;
            } else {
                voxel.flags -= flag_for_current;
            }
        };

        if i > 0 {
            update_adjacencies(
                [i - 1, j, k],
                VoxelFlags::HAS_ADJACENT_X_DN,
                VoxelFlags::HAS_ADJACENT_X_UP,
            );
        }
        if j > 0 {
            update_adjacencies(
                [i, j - 1, k],
                VoxelFlags::HAS_ADJACENT_Y_DN,
                VoxelFlags::HAS_ADJACENT_Y_UP,
            );
        }
        if k > 0 {
            update_adjacencies(
                [i, j, k - 1],
                VoxelFlags::HAS_ADJACENT_Z_DN,
                VoxelFlags::HAS_ADJACENT_Z_UP,
            );
        }
    }

    #[inline]
    fn update_lower_adjacencies_for_empty_voxel(chunk_voxels: &mut [Voxel], [i, j, k]: [usize; 3]) {
        let mut update_adjacencies = |adjacent_indices, flag_for_adjacent| {
            let adjacent_idx = linear_voxel_idx_within_chunk(&adjacent_indices);
            let adjacent_voxel = NonUniformVoxelChunk::get_voxel_mut(chunk_voxels, adjacent_idx);

            adjacent_voxel.remove_flags(flag_for_adjacent);
        };

        if i > 0 {
            update_adjacencies([i - 1, j, k], VoxelFlags::HAS_ADJACENT_X_UP);
        }
        if j > 0 {
            update_adjacencies([i, j - 1, k], VoxelFlags::HAS_ADJACENT_Y_UP);
        }
        if k > 0 {
            update_adjacencies([i, j, k - 1], VoxelFlags::HAS_ADJACENT_Z_UP);
        }
    }

    #[inline]
    fn complete_extracted_voxel_object(
        voxel_extent: f32,
        parent_origin_offset_in_root: &[usize; 3],
        chunk_counts: [usize; 3],
        chunk_ranges: [Range<usize>; 3],
        uniform_chunk_count: usize,
        non_uniform_chunk_count: usize,
        voxels: Vec<Voxel>,
        chunks: Vec<VoxelChunk>,
        split_detector: SplitDetector,
    ) -> Option<ExtractedVoxelObject> {
        assert_eq!(voxels.len(), non_uniform_chunk_count * CHUNK_VOXEL_COUNT);
        assert_eq!(chunks.len(), chunk_counts.iter().product::<usize>());

        // If the extracted object only contains a few non-empty voxels, we
        // discard it to avoid creating a lot of tiny voxel objects
        if uniform_chunk_count == 0 {
            let extracted_non_empty_voxel_count = voxels
                .iter()
                .filter(|voxel| !voxel.is_empty())
                .take(NON_EMPTY_VOXEL_THRESHOLD)
                .count();

            if extracted_non_empty_voxel_count < NON_EMPTY_VOXEL_THRESHOLD {
                return None;
            }
        }

        // Offset in number of voxels from the origin of the original object to
        // the origin of the extracted object
        let origin_offset_in_parent = chunk_ranges.map(|range| range.start * CHUNK_SIZE);

        Some(if chunk_counts.iter().all(|&count| count <= 2) {
            // If the extracted object consists of at most 2 x 2 x 2 chunks,
            // there is a reasonable chance that the actual region of extracted
            // voxels could fit within a single chunk if we offset it
            // appropriately. It is worth doing the extra work of attempting
            // this to avoid creating a lot of unneccesary multi-chunk objects.
            Self::create_extracted_voxel_object_in_single_chunk_if_possible(
                voxel_extent,
                parent_origin_offset_in_root,
                origin_offset_in_parent,
                chunk_counts,
                uniform_chunk_count,
                chunks,
                voxels,
                split_detector,
            )
        } else {
            Self::create_extracted_voxel_object(
                voxel_extent,
                parent_origin_offset_in_root,
                origin_offset_in_parent,
                chunk_counts,
                chunks,
                voxels,
                split_detector,
            )
        })
    }

    #[inline]
    fn create_extracted_voxel_object_in_single_chunk_if_possible(
        voxel_extent: f32,
        parent_origin_offset_in_root: &[usize; 3],
        origin_offset_in_parent: [usize; 3],
        chunk_counts: [usize; 3],
        uniform_chunk_count: usize,
        chunks: Vec<VoxelChunk>,
        voxels: Vec<Voxel>,
        split_detector: SplitDetector,
    ) -> ExtractedVoxelObject {
        // If there uniform chunks, the object either won't fit in a single
        // chunk or it does so already. If it is already a single chunk, we
        // don't have to do any extra work.
        if uniform_chunk_count == 0 && chunk_counts.iter().product::<usize>() > 1 {
            let occupied_voxel_ranges =
                determine_occupied_voxel_ranges(chunk_counts, &chunks, &voxels);

            // We need room for an empty single-voxel boundary to ensure smooth
            // signed distances around the surface if it is close to the
            // boundary
            if occupied_voxel_ranges
                .iter()
                .all(|range| range.len() <= CHUNK_SIZE - 2)
            {
                let origin_offset_within_object = occupied_voxel_ranges
                    .clone()
                    .map(|range| range.start.saturating_sub(1)); // -1 for the empty boundary

                let mut single_chunk_voxels = vec![Voxel::maximally_outside(); CHUNK_VOXEL_COUNT];

                let mut chunk_idx = 0;
                for chunk_i in 0..chunk_counts[0] {
                    for chunk_j in 0..chunk_counts[1] {
                        for chunk_k in 0..chunk_counts[2] {
                            let chunk_indices = [chunk_i, chunk_j, chunk_k];

                            // Copy over the voxels in the sub-block of this
                            // chunk that overlaps our single chunk

                            let single_chunk_ranges_within_chunk: [_; 3] = array::from_fn(|dim| {
                                let start = origin_offset_within_object[dim]
                                    .saturating_sub(chunk_indices[dim] * CHUNK_SIZE)
                                    .min(CHUNK_SIZE);
                                let end = (origin_offset_within_object[dim] + CHUNK_SIZE)
                                    .saturating_sub(chunk_indices[dim] * CHUNK_SIZE)
                                    .min(CHUNK_SIZE);
                                start..end
                            });

                            let overlap_spans = single_chunk_ranges_within_chunk
                                .clone()
                                .map(|range| range.len());

                            let offset_within_single_chunk: [_; 3] = array::from_fn(|dim| {
                                (chunk_indices[dim] * CHUNK_SIZE)
                                    .saturating_sub(origin_offset_within_object[dim])
                            });

                            match &chunks[chunk_idx] {
                                VoxelChunk::NonUniform(NonUniformVoxelChunk {
                                    data_offset,
                                    ..
                                }) => {
                                    let chunk_voxels = chunk_voxels(&voxels, *data_offset);

                                    for i in 0..overlap_spans[0] {
                                        for j in 0..overlap_spans[1] {
                                            for k in 0..overlap_spans[2] {
                                                let indices = [i, j, k];
                                                let src_idx = linear_voxel_idx_within_chunk(
                                                    &array::from_fn(|dim| {
                                                        single_chunk_ranges_within_chunk[dim].start
                                                            + indices[dim]
                                                    }),
                                                );
                                                let dest_idx = linear_voxel_idx_within_chunk(
                                                    &array::from_fn(|dim| {
                                                        offset_within_single_chunk[dim]
                                                            + indices[dim]
                                                    }),
                                                );
                                                single_chunk_voxels[dest_idx] =
                                                    chunk_voxels[src_idx];
                                            }
                                        }
                                    }
                                }
                                VoxelChunk::Void => {}
                                VoxelChunk::Uniform(_) => unreachable!(),
                            }
                            chunk_idx += 1;
                        }
                    }
                }

                let single_chunk_origin_offset_in_parent = array::from_fn(|dim| {
                    origin_offset_in_parent[dim] + origin_offset_within_object[dim]
                });

                let face_distributions = array::from_fn(|dim| {
                    let lower =
                        if origin_offset_within_object[dim] == occupied_voxel_ranges[dim].start {
                            FaceVoxelDistribution::Mixed
                        } else {
                            FaceVoxelDistribution::Empty
                        };
                    let upper = if origin_offset_within_object[dim] + CHUNK_SIZE
                        == occupied_voxel_ranges[dim].end
                    {
                        FaceVoxelDistribution::Mixed
                    } else {
                        FaceVoxelDistribution::Empty
                    };
                    [lower, upper]
                });

                let mut single_chunk = NonUniformVoxelChunk {
                    data_offset: 0,
                    face_distributions,
                    flags: VoxelChunkFlags::empty(),
                    split_detection: NonUniformChunkSplitDetectionData::new(),
                };

                // The chunks before packing may have had unresolved boundary
                // adjacencies, and now those are in the interior of the single
                // chunk
                single_chunk.update_internal_adjacencies(&mut single_chunk_voxels);

                // Since the voxels are now chunked differently, we need a new
                // split detector
                let mut single_chunk_split_detector = SplitDetector::new(0, 1);
                single_chunk_split_detector.update_local_connected_regions_for_chunk(
                    &single_chunk_voxels,
                    &mut single_chunk,
                    0,
                );

                return Self::create_extracted_voxel_object(
                    voxel_extent,
                    parent_origin_offset_in_root,
                    single_chunk_origin_offset_in_parent,
                    [1; 3],
                    vec![VoxelChunk::NonUniform(single_chunk)],
                    single_chunk_voxels,
                    single_chunk_split_detector,
                );
            }
        }

        Self::create_extracted_voxel_object(
            voxel_extent,
            parent_origin_offset_in_root,
            origin_offset_in_parent,
            chunk_counts,
            chunks,
            voxels,
            split_detector,
        )
    }

    fn create_extracted_voxel_object(
        voxel_extent: f32,
        parent_origin_offset_in_root: &[usize; 3],
        origin_offset_in_parent: [usize; 3],
        chunk_counts: [usize; 3],
        chunks: Vec<VoxelChunk>,
        voxels: Vec<Voxel>,
        split_detector: SplitDetector,
    ) -> ExtractedVoxelObject {
        // The chunk grid of the extracted object should start at the origin
        let offset_chunk_ranges = chunk_counts.map(|count| 0..count);

        let chunk_idx_strides = [chunk_counts[2] * chunk_counts[1], chunk_counts[2], 1];

        let offset_voxel_ranges = offset_chunk_ranges
            .clone()
            .map(|chunk_range| chunk_range.start * CHUNK_SIZE..chunk_range.end * CHUNK_SIZE);

        let origin_offset_in_root =
            array::from_fn(|dim| parent_origin_offset_in_root[dim] + origin_offset_in_parent[dim]);

        let voxel_object = Self {
            voxel_extent,
            inverse_voxel_extent: voxel_extent.recip(),
            chunk_counts,
            chunk_idx_strides,
            occupied_chunk_ranges: offset_chunk_ranges,
            occupied_voxel_ranges: offset_voxel_ranges,
            origin_offset_in_root,
            chunks,
            voxels,
            split_detector,
            invalidated_mesh_chunk_indices: HashSet::default(),
        };

        ExtractedVoxelObject {
            voxel_object,
            origin_offset_in_parent,
        }
    }
}

impl PropertyTransferrer for NoPropertyTransferrer {
    fn transfer_voxel(&mut self, _object_voxel_indices: &[usize; 3], _voxel: Voxel) {}

    fn transfer_non_uniform_chunk(&mut self, _chunk_indices: &[usize; 3], _chunk_voxels: &[Voxel]) {
    }

    fn transfer_uniform_chunk(&mut self, _chunk_indices: &[usize; 3], _chunk_voxel: Voxel) {}
}

impl PropertyComputer for NoPropertyComputer {
    fn compute_for_voxel(&mut self, _object_voxel_indices: &[usize; 3], _voxel: Voxel) {}

    fn compute_for_non_uniform_chunk(
        &mut self,
        _chunk_indices: &[usize; 3],
        _chunk_voxels: &[Voxel],
    ) {
    }

    fn compute_for_uniform_chunk(&mut self, _chunk_indices: &[usize; 3], _chunk_voxel: Voxel) {}
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::{
        chunks::inertia::VoxelObjectInertialPropertyManager, generation::SDFVoxelGenerator,
    };
    use approx::assert_relative_eq;
    use arbitrary::{Arbitrary, Result, Unstructured};
    use bytemuck::{Pod, Zeroable};
    use impact_alloc::Global;
    use impact_math::vector::Vector3;
    use impact_tesselation::{delaunay::DelaunayTetrahedralization, voronoi::VoronoiPolyhedron};

    const FLOAT_RESOLUTION: u32 = 10000;
    const DELAUNAY_DOMAIN_EXTENT: f32 = 200.0;

    #[derive(Clone, Debug, Arbitrary)]
    pub struct CopyPolyhedronInput {
        generator: SDFVoxelGenerator<Global>,
        points: Vec<DelaunayPoint>,
    }

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Zeroable, Pod)]
    struct DelaunayPoint(Point3C);

    impl Arbitrary<'_> for DelaunayPoint {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let x =
                DELAUNAY_DOMAIN_EXTENT * arbitrary_norm_f32(u)? - (0.5 * DELAUNAY_DOMAIN_EXTENT);
            let y =
                DELAUNAY_DOMAIN_EXTENT * arbitrary_norm_f32(u)? - (0.5 * DELAUNAY_DOMAIN_EXTENT);
            let z =
                DELAUNAY_DOMAIN_EXTENT * arbitrary_norm_f32(u)? - (0.5 * DELAUNAY_DOMAIN_EXTENT);
            Ok(Self(Point3C::new(x, y, z)))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 3 * mem::size_of::<u32>();
            (size, Some(size))
        }
    }

    pub fn fuzz_test_voxel_object_split_off_disconnected_region(
        generator: SDFVoxelGenerator<Global>,
    ) {
        let mut object = ChunkedVoxelObject::generate(&generator);
        let voxel_type_densities = vec![1.0; 256];

        let original_inertial_property_manager =
            VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities);

        let mut inertial_property_manager = original_inertial_property_manager.clone();
        let mut disconnected_inertial_property_manager =
            VoxelObjectInertialPropertyManager::zeroed();

        let mut inertial_property_transferrer = inertial_property_manager.begin_transfer_to(
            &mut disconnected_inertial_property_manager,
            object.voxel_extent(),
            &voxel_type_densities,
        );

        let original_region_count = object.count_regions();
        if let Some(disconnected_object) = object
            .extract_any_disconnected_region_with_property_transferrer(
                &mut inertial_property_transferrer,
            )
        {
            let ExtractedVoxelObject {
                voxel_object: disconnected_object,
                origin_offset_in_parent: origin_offset,
            } = disconnected_object;

            assert!(original_region_count > 1);
            assert_eq!(disconnected_object.count_regions(), 1);
            assert_eq!(object.count_regions(), original_region_count - 1);

            assert!(!disconnected_object.is_effectively_empty());
            disconnected_object.validate_adjacencies();
            disconnected_object.validate_chunk_obscuredness();
            disconnected_object.validate_sdf();
            disconnected_object.validate_region_count();

            object.validate_adjacencies();
            object.validate_chunk_obscuredness();
            object.validate_sdf();
            object.validate_region_count();

            assert_relative_eq!(
                &inertial_property_manager.add(&disconnected_inertial_property_manager),
                &original_inertial_property_manager,
                epsilon = 1e-8,
                max_relative = 1e-8,
            );

            disconnected_inertial_property_manager.offset_reference_point_by(&Vector3::from(
                origin_offset.map(|offset| offset as f32 * object.voxel_extent()),
            ));

            disconnected_inertial_property_manager
                .validate_for_object(&disconnected_object, &voxel_type_densities);
        }

        inertial_property_manager.validate_for_object(&object, &voxel_type_densities);
    }

    pub fn fuzz_test_voxel_object_copy_polyhedron(input: CopyPolyhedronInput) {
        let object = ChunkedVoxelObject::generate(&input.generator);
        let voxel_type_densities = [1.0; 256];

        let aabb = object.compute_normalized_chunk_grid_bounds();

        let points = bytemuck::cast_slice(&input.points);
        let tetrahedralization = DelaunayTetrahedralization::construct(points).unwrap();

        let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
            let Some(polyhedron_aabb) = polyhedron.compute_bounded_aabb(&aabb) else {
                continue;
            };

            let mut poly_inertial_property_manager = VoxelObjectInertialPropertyManager::zeroed();
            let mut inertial_property_copier = poly_inertial_property_manager
                .begin_computation(object.voxel_extent(), &voxel_type_densities);

            let Some(copied_object) = object.copy_polyhedron_with_property_computer(
                &polyhedron_aabb,
                &polyhedron.face_planes,
                &mut inertial_property_copier,
            ) else {
                continue;
            };

            let ExtractedVoxelObject {
                voxel_object: poly_object,
                origin_offset_in_parent: origin_offset,
            } = copied_object;

            poly_object.validate_adjacencies();
            poly_object.validate_chunk_obscuredness();
            poly_object.validate_sdf();
            poly_object.validate_region_count();
            assert!(!poly_object.is_effectively_empty());

            poly_inertial_property_manager.offset_reference_point_by(&Vector3::from(
                origin_offset.map(|offset| offset as f32 * object.voxel_extent()),
            ));

            poly_inertial_property_manager.validate_for_object(&poly_object, &voxel_type_densities);
        }
    }

    fn arbitrary_norm_f32(u: &mut Unstructured<'_>) -> Result<f32> {
        Ok((f64::from(u.int_in_range(0..=FLOAT_RESOLUTION)?) / f64::from(FLOAT_RESOLUTION)) as f32)
    }
}

#[cfg(not(miri))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generation::{
            SDFVoxelGenerator,
            sdf::{SDFGraph, SDFNode},
            voxel_type::SameVoxelTypeGenerator,
        },
        voxel_types::VoxelType,
    };
    use impact_alloc::Global;
    use impact_math::vector::Vector3;

    #[test]
    fn connected_region_count_is_correct_for_single_voxel() {
        let mut graph = SDFGraph::new_in(Global);
        graph.add_node(SDFNode::new_box([1.0; 3]));
        let sdf_generator = graph.build_in(Global).unwrap();

        let generator = SDFVoxelGenerator::new(
            1.0,
            sdf_generator,
            SameVoxelTypeGenerator::new(VoxelType::default()).into(),
        );

        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_region_count();
    }

    #[test]
    fn should_split_off_disconnected_sphere() {
        let mut graph = SDFGraph::new_in(Global);
        let sphere_1_id = graph.add_node(SDFNode::new_sphere(25.0));
        let sphere_2_id = graph.add_node(SDFNode::new_sphere(25.0));
        let sphere_2_id = graph.add_node(SDFNode::new_translation(
            sphere_2_id,
            Vector3::new(60.0, 0.0, 0.0),
        ));
        graph.add_node(SDFNode::new_union(sphere_1_id, sphere_2_id, 1.0));
        let sdf_generator = graph.build_in(Global).unwrap();

        let generator = SDFVoxelGenerator::new(
            1.0,
            sdf_generator,
            SameVoxelTypeGenerator::new(VoxelType::default()).into(),
        );
        let mut object = ChunkedVoxelObject::generate(&generator);
        assert!(object.extract_any_disconnected_region().is_some());
    }
}
