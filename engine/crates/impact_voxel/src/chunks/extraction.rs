//! Extraction of regions of voxel objects.

use crate::{
    Voxel,
    chunks::{
        CHUNK_SIZE, CHUNK_VOXEL_COUNT, ChunkedVoxelObject, FaceVoxelDistribution,
        NON_EMPTY_VOXEL_THRESHOLD, NonUniformVoxelChunk, VoxelChunk, VoxelChunkFlags, chunk_voxels,
        chunk_voxels_mut, determine_occupied_voxel_ranges, linear_voxel_idx_within_chunk,
        split_detection::{
            CHUNK_MAX_REGIONS, GlobalRegionLabel, NonUniformChunkSplitDetectionData, SplitDetector,
            chunk_voxel_region_labels, find_root_for_region, non_uniform_chunk_regions,
            non_uniform_chunk_start_region_idx,
        },
    },
};
use impact_containers::HashSet;
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
        extracted.voxel_object.update_occupied_voxel_ranges();

        Some(extracted)
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
        if uniform_chunk_count == 0 && non_uniform_chunk_count <= 8 {
            let disconnected_non_empty_voxel_count =
                voxels.iter().filter(|voxel| !voxel.is_empty()).count();

            if disconnected_non_empty_voxel_count < NON_EMPTY_VOXEL_THRESHOLD {
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

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::{
        chunks::inertia::VoxelObjectInertialPropertyManager, generation::SDFVoxelGenerator,
    };
    use approx::assert_relative_eq;
    use impact_alloc::Global;
    use impact_math::vector::Vector3;

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
