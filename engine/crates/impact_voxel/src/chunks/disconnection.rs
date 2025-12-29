//! Detection of disconnected regions in voxel objects.

use super::{
    FaceVoxelDistribution, chunk_voxels, chunk_voxels_mut, determine_occupied_voxel_ranges,
    extract_slice_segments_mut,
};
use crate::{
    Voxel, VoxelFlags,
    chunks::{
        CHUNK_SIZE, CHUNK_VOXEL_COUNT, ChunkedVoxelObject, LOG2_CHUNK_SIZE, LoopForChunkVoxels,
        NON_EMPTY_VOXEL_THRESHOLD, NonUniformVoxelChunk, UniformVoxelChunk, VoxelChunk,
        VoxelChunkFlags, chunk_start_voxel_idx, chunk_voxel_indices_from_linear_idx,
        linear_voxel_idx_within_chunk,
    },
    utils::{DataLoop3, Dimension, Side},
};
use cfg_if::cfg_if;
use impact_containers::HashSet;
use std::{array, cmp::Ordering, iter, mem, ops::Range};

/// Represents a helper for keeping track of the transferral of some aggregate
/// voxel property when a voxel object is split into multiple objects.
pub trait PropertyTransferrer {
    fn transfer_voxel(&mut self, object_voxel_indices: &[usize; 3], voxel: Voxel);

    fn transfer_non_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxels: &[Voxel]);

    fn transfer_uniform_chunk(&mut self, chunk_indices: &[usize; 3], chunk_voxel: Voxel);
}

/// A [`ChunkedVoxelObject`] that has been disconnected from a larger object.
#[derive(Clone, Debug)]
pub struct DisconnectedVoxelObject {
    /// The disconnected object.
    pub voxel_object: ChunkedVoxelObject,
    /// The offset in whole voxels from the origin of the parent object to the
    /// origin of the disconnected object, in the reference frame of the
    /// parent object (the disconnected object has the same orientation as the
    /// parent object, only the offset is different).
    pub origin_offset_in_parent: [usize; 3],
}

/// Auxiliary data structure for finding regions of connected voxels in a
/// [`ChunkedVoxelObject`], and hence determining whether and where the object
/// is split into multiple disconnected parts.
///
/// This is a connected-component labeling problem. We solve it in a two-level
/// process, where we first use the voxel adjacency information to label
/// connected regions within each chunk using a Disjoint Set Forest. We then
/// determine the connectivity of these regions across chunk boundaries, and use
/// this information to resolve the globally connected regions, again with the
/// help of a Disjoint Set Forest. This approach allows for efficent
/// recomputation of the connected regions when the voxel object has been
/// modified, because the internal connected regions and the connectivity with
/// adjacent chunks only have to be recomputed for the specific chunks that
/// changed. The global resolution pass still has to be performed every time,
/// but this is relatively efficient since it operates on chunks rather than
/// individual voxels.
///
/// This two-level approach was proposed by Sean Barrett
/// (<https://stb.handmade.network/blog/p/1136-connected_components_algorithm#6475>),
/// and this implementation is based on his code.
#[derive(Clone, Debug)]
pub struct SplitDetector {
    /// Labels identifying which chunk-local connected region each voxel belongs
    /// to. This buffer is laid out exactly like `voxels` in
    /// `ChunkedVoxelObject`, with the voxels for each non-uniform chunk in a
    /// contiguous section.
    voxel_region_labels: Vec<LocalRegionLabel>,
    /// Chunk-local connected regions. The first `original_uniform_chunk_count`
    /// regions are the single regions for each uniform chunk. The remainder of
    /// the buffer consists of segments of `CHUNK_MAX_REGIONS` regions, one
    /// segment for each non-uniform chunk.
    regions: Vec<LocalRegion>,
    original_uniform_chunk_count: usize,
    /// Connections between pairs of chunk-local regions across chunk
    /// boundaries. This buffer is only for outward connections from regions in
    /// non-uniform chunks. It consists of segments of
    /// `CHUNK_MAX_ADJACENT_REGION_CONNECTIONS` connections, one segment for
    /// each non-uniform chunk.
    adjacent_region_connections: Vec<AdjacentRegionConnection>,
    /// Connections between pairs of chunk-local regions across chunk
    /// boundaries. This buffer is only for outward connections from the single
    /// regions in uniform chunks. It consists of segments of
    /// `CHUNK_MAX_ADJACENT_REGION_CONNECTIONS` connections, one segment for
    /// each uniform chunk. We don't bother truncating or modifying the
    /// buffer when a uniform chunk is converted to non-uniform, we just
    /// keep the stale entries around.
    uniform_chunk_adjacencent_region_connections: Vec<AdjacentRegionConnection>,
}

/// Data for a uniform chunk needed to perform split detection.
#[derive(Clone, Copy, Debug, Default)]
pub struct UniformChunkSplitDetectionData {
    /// The index of this uniform chunk in the original list of all uniform
    /// chunks.
    data_offset: u32,
}

/// Data for a non-uniform chunk needed to perform split detection.
#[derive(Clone, Copy, Debug)]
pub struct NonUniformChunkSplitDetectionData {
    /// The total number of local connected regions in this chunk.
    region_count: LocalRegionCount,
    /// The number of local connected regions in this chunk that touch the chunk
    /// boundary.
    boundary_region_count: LocalRegionCount,
}

/// Helper for updating the connections from the local regions of a non-uniform
/// chunk to the local regions in its non-uniform neighbor.
#[derive(Debug)]
pub struct NonUniformChunkConnectionUpdater<'a> {
    current_chunk_voxel_region_labels: &'a [LocalRegionLabel],
    adjacent_chunk_voxel_region_labels: &'a [LocalRegionLabel],
    current_chunk_regions: &'a mut [LocalRegion],
    current_chunk_adjacent_region_connections: &'a mut [AdjacentRegionConnection],
    max_adjacent_region_connections_per_region: LocalRegionCount,
    face_dim: Dimension,
    face_side: Side,
    regions_connected:
        [[u8; CHUNK_MAX_ADJACENT_REGION_CONNECTIONS / 8]; CHUNK_MAX_ADJACENT_REGION_CONNECTIONS],
}

/// A chunk-local region of connected voxels.
#[derive(Clone, Copy, Debug)]
struct LocalRegion {
    /// A globally unique label pointing to some local region that is part of
    /// the same global region as this region. If that region is just this
    /// one (it points to itself), this region is the representative local
    /// region for the global region, and all other local regions in the
    /// global region eventually point to this one.
    parent_label: GlobalRegionLabel,
    /// The start index of this local region's subsection of its chunk's section
    /// of the `adjacent_region_connections` or
    /// `uniform_chunk_adjacencent_regions` buffer.
    adjacent_region_connection_start_idx: LocalRegionCount,
    /// The length of this local region's subsection of its chunk's section
    /// of the `adjacent_region_connections` or
    /// `uniform_chunk_adjacencent_regions` buffer.
    adjacent_region_connection_count: LocalRegionCount,
}

/// Identifier for a connected voxel region within a chunk.
type LocalRegionLabel = u8;

/// Counter for [`LocalRegionLabel`]s.
type LocalRegionCount = u16;

/// Label that ties together local voxel regions that make up a single connected
/// voxel region within the whole voxel object. The value encodes the index of a
/// chunk and the index of a local region within that chunk, identifying a
/// particular local region globally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalRegionLabel(u32);

/// A one-sided connection from a local region in one chunk to another
/// local region in an adjacent chunk. The value encodes the [`Dimension`] and
/// [`Side`] of the face of the chunk that the connection crosses, as well as
/// the index of the local region connected to in the adjacent chunk.
#[derive(Clone, Copy, Debug)]
struct AdjacentRegionConnection(u16);

struct NoPropertyTransferrer;

/// The maximum number of [`LocalRegion`]s in a chunk.
const CHUNK_MAX_REGIONS: usize = 1 << LOG2_MAX_REGIONS_PER_CHUNK;

/// The theoretical maximum in log2 is `3 * LOG2_CHUNK_SIZE - 1`, corresponding
/// to a checkerboard voxel pattern. This is an extremely unrealistic
/// configration in practice, so we subtract 3 from this value, reducing the
/// maximum number we support by a factor of 8.
const LOG2_MAX_REGIONS_PER_CHUNK: usize = (3 * LOG2_CHUNK_SIZE - 1) - 3;

/// The [`LocalRegionLabel`] assigned to all empty voxels.
const EMPTY_VOXEL_LABEL: LocalRegionLabel = LocalRegionLabel::MAX;

// This is required for `LocalRegionLabel` to fit in a `u8`
const _: () = assert!(CHUNK_MAX_REGIONS - 1 <= u8::MAX as usize);

/// The maximum number of outgoing [`AdjacentRegionConnection`]s from all
/// [`LocalRegion`]s in a chunk.
///
/// If the face has a checkerboard pattern of voxels, each being in a separate
/// region, that would be the maximum number of regions the face can contain. In
/// that case, each region can only connect to one adjacent region across that
/// face, so the maximum number of connections is just the number of regions on
/// the face. Considering the other extreme, the maximum number of separate
/// connections a region could have across the face would be if the region
/// covers the face and there is a checkerboard pattern on the other side. In
/// that case, there would only be room for one region on the face, so the
/// maximum total number of connections across the face would be the same:
/// `CHUNK_SIZE.pow(2) / 2`. We would then multiply by 6 to get the maximum
/// number of adjacent region connections for the whole chunk. Because of the
/// duality described above, this is equivalent to the maximum number of
/// boundary regions for the chunk.
///
/// Since we are already more a lot more restrictive than the theoretical
/// maximum for [`CHUNK_MAX_REGIONS`], and most regions in a chunk are very
/// likely to touch the boundary, we will go ahead an use the same value for
/// `CHUNK_MAX_ADJACENT_REGION_CONNECTIONS` as for `CHUNK_MAX_REGIONS`.
const CHUNK_MAX_ADJACENT_REGION_CONNECTIONS: usize = CHUNK_MAX_REGIONS;

/// The maximum number of [`LocalRegion`]s on a chunk that may touch a boundary.
/// This is equivalent to [`CHUNK_MAX_ADJACENT_REGION_CONNECTIONS`].
const CHUNK_MAX_BOUNDARY_REGIONS: usize = CHUNK_MAX_ADJACENT_REGION_CONNECTIONS;

impl ChunkedVoxelObject {
    /// Checks if the object consists of more than one disconnected region, and
    /// if so, splits off one of them into a seperate object and returns it.
    /// Both this object and the returned object will have the correct derived
    /// state when this call returns.
    pub fn split_off_any_disconnected_region(&mut self) -> Option<DisconnectedVoxelObject> {
        self.split_off_any_disconnected_region_with_property_transferrer(&mut NoPropertyTransferrer)
    }

    /// Checks if the object consists of more than one disconnected region, and
    /// if so, splits off one of them into a seperate object and returns it.
    /// Both this object and the returned object will have the correct derived
    /// state when this call returns. The methods of the given
    /// `PropertyTransferrer` will be called appropriately when voxels or whole
    /// chunks are copied over to the disconnected object.
    pub fn split_off_any_disconnected_region_with_property_transferrer(
        &mut self,
        property_transferrer: &mut impl PropertyTransferrer,
    ) -> Option<DisconnectedVoxelObject> {
        // If we just look for any disconnected region and split it off, that region
        // could turn out to contain most of the object. To avoid this, we consider two
        // regions in tandem and split off whichever of them contains the fewest chunks.

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
                                    // Make sure all local regions pointing to either of our
                                    // disconnected regions store their root label directly, so that
                                    // we don't have to call `find_root_for_region` again in
                                    // `Self::split_off_region`
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
                        VoxelChunk::Empty => {}
                    }
                }
            }
        }

        // We prioritize having the smallest number of non-uniform chunks in the object
        // that we split off
        let smallest_region_idx =
            match region_non_uniform_chunk_counts[0].cmp(&region_non_uniform_chunk_counts[1]) {
                Ordering::Less => 0,
                Ordering::Greater => 1,
                Ordering::Equal => {
                    // Use total non-empty chunk counts to break tie
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

        self.split_off_region(
            smallest_region,
            smallest_region_linear_chunk_indices,
            smallest_non_uniform_chunk_count,
            smallest_region_chunk_ranges,
            property_transferrer,
        )
    }

    fn split_off_region(
        &mut self,
        disconnected_region: GlobalRegionLabel,
        region_linear_chunk_indices: Vec<usize>,
        region_non_uniform_chunk_count: usize,
        region_chunk_ranges: [Range<usize>; 3],
        property_transferrer: &mut impl PropertyTransferrer,
    ) -> Option<DisconnectedVoxelObject> {
        let region_chunk_counts = region_chunk_ranges.clone().map(|range| range.len());
        let total_region_chunk_count = region_chunk_counts.iter().product();
        let region_uniform_chunk_count =
            region_linear_chunk_indices.len() - region_non_uniform_chunk_count;

        let mut region_voxels =
            Vec::with_capacity(region_non_uniform_chunk_count * CHUNK_VOXEL_COUNT);

        let mut region_chunks = Vec::with_capacity(total_region_chunk_count);

        let mut region_split_detector =
            SplitDetector::new(region_uniform_chunk_count, region_non_uniform_chunk_count);

        // We use this to lookup if a `LocalRegionLabel` for a voxel corresponds to the
        // global region that we are splitting off
        let mut split_off_voxel_at_label = [false; CHUNK_MAX_REGIONS];

        // This keeps track of how far we have gotten in `region_linear_chunk_indices`
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
                                // regions than the one we want to split off
                                let mut is_mixed = false;
                                for region_idx in 0..chunk.split_detection.region_count as usize {
                                    // We made sure to put the root as the parent in
                                    // `Self::split_off_any_disconnected_region`
                                    let region_root_label = chunk_regions[region_idx].parent_label;

                                    split_off_voxel_at_label[region_idx] =
                                        region_root_label == disconnected_region;

                                    is_mixed = is_mixed || region_root_label != disconnected_region;
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
                                                let label = labels[voxel_idx];

                                                let region_voxel = if voxel.is_empty() {
                                                    // Since the signed distances of empty voxels
                                                    // adjacent to non-empty ones affect meshing, we
                                                    // copy over empty voxels unconditionally
                                                    *voxel
                                                } else if split_off_voxel_at_label[label as usize] {
                                                    // The voxel belongs to the region we are
                                                    // splitting off, so we grab it and replace it
                                                    // with an empty voxel in the original object
                                                    let region_voxel = *voxel;

                                                    property_transferrer
                                                        .transfer_voxel(&[i, j, k], region_voxel);

                                                    *voxel = Voxel::maximally_outside();

                                                    region_voxel
                                                } else {
                                                    // The voxel belongs to some other region, so we
                                                    // write an empty voxel to the splitted off
                                                    // object
                                                    Voxel::maximally_outside()
                                                };
                                                region_voxels.push(region_voxel);

                                                voxel_idx += 1;
                                            }
                                        }
                                    }

                                    // We have filled this chunk of the splitted off object, so we
                                    // go ahead and compute the face distributions and internal
                                    // adjacencies for the chunk
                                    region_chunk
                                        .update_face_distributions_and_internal_adjacencies_and_count_non_empty_voxels(
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
                                    // are splitting off, we can copy over all the voxels in one go
                                    region_voxels.extend_from_slice(chunk_voxels);

                                    // We replace them with empty voxels and mark the original chunk
                                    // as empty (although the voxels now lose their owner, we might
                                    // still encounter them when looping over all voxels for things
                                    // like aggregations, so it is still important that we make them
                                    // empty)
                                    chunk_voxels.fill(Voxel::maximally_outside());
                                    self.chunks[chunk_idx] = VoxelChunk::Empty;

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
                                    chunk.update_face_distributions_and_internal_adjacencies_and_count_non_empty_voxels(
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
                                // are splitting off will be separated from the modified voxels by
                                // empty space, so their meshes should not be affected. The adjacent
                                // chunks that are a part of the region we are splitting off will be
                                // or have already been visited in this loop, so we don't have to
                                // address them here.
                                self.invalidated_mesh_chunk_indices.insert(chunk_indices);
                            }
                            VoxelChunk::Uniform(mut chunk) => {
                                // If the chunk to split off is uniform, we can just move it over
                                // and replace it with an empty chunk, after making sure to
                                // overwrite the data offset with the correct value for the
                                // disconnected object

                                property_transferrer
                                    .transfer_uniform_chunk(&chunk_indices, chunk.voxel);

                                chunk.split_detection.data_offset =
                                    region_uniform_chunk_data_offset;
                                region_uniform_chunk_data_offset += 1;

                                region_chunks.push(VoxelChunk::Uniform(chunk));
                                self.chunks[chunk_idx] = VoxelChunk::Empty;
                            }
                            VoxelChunk::Empty => {
                                unreachable!()
                            }
                        }
                        cursor += 1;
                    } else {
                        // We need to pad with empty chunks to fill up the whole chunk grid of the
                        // splitted of object
                        region_chunks.push(VoxelChunk::Empty);
                    }
                }
            }
        }

        assert_eq!(
            region_voxels.len(),
            region_non_uniform_chunk_count * CHUNK_VOXEL_COUNT
        );
        assert_eq!(region_chunks.len(), total_region_chunk_count);

        // Now that we have removed part of the original object, we may be able to
        // shrink the recorded occupied chunk and voxel ranges
        self.update_occupied_ranges();

        // Any chunk within or adjacent to the block of chunks covering the disconnected
        // region may have invalidated adjacencies
        self.update_upper_boundary_adjacencies_for_chunks_in_ranges(
            region_chunk_ranges
                .clone()
                .map(|range| range.start.saturating_sub(1)..range.end),
        );

        // We also have to resolve the globally connected regions now that the original
        // object has been modified
        self.resolve_connected_regions_between_all_chunks();

        // If the disconnected region only contains a few non-empty voxels, we discard
        // the disconnected object to avoid creating a lot of tiny voxel objects
        if region_uniform_chunk_count == 0 && region_non_uniform_chunk_count <= 8 {
            let disconnected_non_empty_voxel_count = region_voxels
                .iter()
                .filter(|voxel| !voxel.is_empty())
                .count();

            if disconnected_non_empty_voxel_count < NON_EMPTY_VOXEL_THRESHOLD {
                return None;
            }
        }

        // Offset in number of voxels from the origin of the original object to the
        // origin of the disconnected object
        let origin_offset_in_parent = region_chunk_ranges.map(|range| range.start * CHUNK_SIZE);

        Some(if region_chunk_counts.iter().all(|&count| count <= 2) {
            // If the disconnected object consists of at most 2 x 2 x 2 chunks, there is a
            // reasonable chance that the actual region of disconnected voxels could fit
            // within a single chunk if we offset it appropriately. It is worth doing the
            // extra work of attempting this to avoid creating a lot of unneccesary
            // multi-chunk objects.
            Self::create_disconnected_voxel_object_in_single_chunk_if_possible(
                self.voxel_extent,
                &self.origin_offset_in_root,
                origin_offset_in_parent,
                region_chunk_counts,
                region_uniform_chunk_count,
                region_chunks,
                region_voxels,
                region_split_detector,
            )
        } else {
            Self::create_disconnected_voxel_object(
                self.voxel_extent,
                &self.origin_offset_in_root,
                origin_offset_in_parent,
                region_chunk_counts,
                region_chunks,
                region_voxels,
                region_split_detector,
            )
        })
    }

    fn create_disconnected_voxel_object_in_single_chunk_if_possible(
        voxel_extent: f32,
        parent_origin_offset_in_root: &[usize; 3],
        origin_offset_in_parent: [usize; 3],
        chunk_counts: [usize; 3],
        uniform_chunk_count: usize,
        chunks: Vec<VoxelChunk>,
        voxels: Vec<Voxel>,
        split_detector: SplitDetector,
    ) -> DisconnectedVoxelObject {
        // If there uniform chunks, the object either won't fit in a single chunk or it
        // does so already. If it is already a single chunk, we don't have to do any
        // extra work.
        if uniform_chunk_count == 0 && chunk_counts.iter().product::<usize>() > 1 {
            let occupied_voxel_ranges =
                determine_occupied_voxel_ranges(chunk_counts, &chunks, &voxels);

            // We need room for an empty single-voxel boundary to ensure smooth signed
            // distances around the surface if it is close to the boundary
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

                            // Copy over the voxels in the sub-block of this chunk that overlaps our
                            // single chunk

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
                                VoxelChunk::Empty => {}
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

                // Since the voxels are now chunked differently, we need a new split detector
                let mut single_chunk_split_detector = SplitDetector::new(0, 1);
                single_chunk_split_detector.update_local_connected_regions_for_chunk(
                    &single_chunk_voxels,
                    &mut single_chunk,
                    0,
                );

                return Self::create_disconnected_voxel_object(
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

        Self::create_disconnected_voxel_object(
            voxel_extent,
            parent_origin_offset_in_root,
            origin_offset_in_parent,
            chunk_counts,
            chunks,
            voxels,
            split_detector,
        )
    }

    fn create_disconnected_voxel_object(
        voxel_extent: f32,
        parent_origin_offset_in_root: &[usize; 3],
        origin_offset_in_parent: [usize; 3],
        chunk_counts: [usize; 3],
        chunks: Vec<VoxelChunk>,
        voxels: Vec<Voxel>,
        split_detector: SplitDetector,
    ) -> DisconnectedVoxelObject {
        // The chunk grid of the disconnected object should start at the origin
        let offset_chunk_ranges = chunk_counts.map(|count| 0..count);

        let chunk_idx_strides = [chunk_counts[2] * chunk_counts[1], chunk_counts[2], 1];

        let offset_voxel_ranges = offset_chunk_ranges
            .clone()
            .map(|chunk_range| chunk_range.start * CHUNK_SIZE..chunk_range.end * CHUNK_SIZE);

        let origin_offset_in_root =
            array::from_fn(|dim| parent_origin_offset_in_root[dim] + origin_offset_in_parent[dim]);

        let mut voxel_object = Self {
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

        // We have already computed the internal adjacencies and local region
        // connectivity in the disconnected object, but all derived state between chunks
        // must be computed from scratch
        voxel_object.compute_all_chunk_external_derived_state();

        // Also make sure to tighten the voxel ranges
        voxel_object.update_occupied_voxel_ranges();

        DisconnectedVoxelObject {
            voxel_object,
            origin_offset_in_parent,
        }
    }

    /// Identifies two disconnected regions of the voxel object (if there are
    /// more than two, the rest are ignored). Returns [`None`] if the object has
    /// fewer than two disconnected regions.
    ///
    /// Assumes that [`Self::resolve_connected_regions_between_all_chunks`] has
    /// been called after the object was last modified.
    pub fn find_two_disconnected_regions(&self) -> Option<[GlobalRegionLabel; 2]> {
        let regions = &self.split_detector.regions;

        let mut region_count = 0;
        let mut disconnected_regions = [GlobalRegionLabel::zero(); 2];

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
                            let chunk_regions = &regions[start_region_idx
                                ..start_region_idx + chunk_data.region_count as usize];

                            for (region_idx, region) in chunk_regions.iter().enumerate() {
                                if region.parent_label
                                    == GlobalRegionLabel::new(chunk_idx as u32, region_idx as u32)
                                {
                                    disconnected_regions[region_count] = region.parent_label;
                                    region_count += 1;
                                    if region_count > 1 {
                                        return Some(disconnected_regions);
                                    }
                                }
                            }
                        }
                        VoxelChunk::Uniform(chunk) => {
                            let region = &regions[chunk.split_detection.data_offset as usize];
                            let region_idx = 0;
                            if region.parent_label
                                == GlobalRegionLabel::new(chunk_idx as u32, region_idx)
                            {
                                disconnected_regions[region_count] = region.parent_label;
                                region_count += 1;
                                if region_count > 1 {
                                    return Some(disconnected_regions);
                                }
                            }
                        }
                        VoxelChunk::Empty => {}
                    }
                }
            }
        }

        None
    }

    /// Counts the total number of disconnected regions that the voxel object
    /// consist of.
    ///
    /// Assumes that [`Self::resolve_connected_regions_between_all_chunks`] has
    /// been called after the object was last modified.
    pub fn count_regions(&self) -> usize {
        let regions = &self.split_detector.regions;

        let mut region_count = 0;

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
                            let chunk_regions = &regions[start_region_idx
                                ..start_region_idx + chunk_data.region_count as usize];

                            for (region_idx, region) in chunk_regions.iter().enumerate() {
                                if region.parent_label
                                    == GlobalRegionLabel::new(chunk_idx as u32, region_idx as u32)
                                {
                                    region_count += 1;
                                }
                            }
                        }
                        VoxelChunk::Uniform(chunk) => {
                            let region = &regions[chunk.split_detection.data_offset as usize];
                            let region_idx = 0;
                            if region.parent_label
                                == GlobalRegionLabel::new(chunk_idx as u32, region_idx)
                            {
                                region_count += 1;
                            }
                        }
                        VoxelChunk::Empty => {}
                    }
                }
            }
        }

        region_count
    }

    /// Analyzes each non-uniform chunk to determine all regions of connected
    /// voxels within the chunk.
    pub fn update_local_connected_regions_for_all_chunks(&mut self) {
        for (chunk_idx, chunk) in self.chunks.iter_mut().enumerate() {
            if let VoxelChunk::NonUniform(chunk) = chunk {
                self.split_detector
                    .update_local_connected_regions_for_chunk(
                        &self.voxels,
                        chunk,
                        chunk_idx as u32,
                    );
            }
        }
    }

    /// Analyzes the connection information between the local regions of all
    /// chunks to group the local regions into globally connected regions.
    ///
    /// This should be called whenever any local regions or connections between
    /// them have changed.
    pub fn resolve_connected_regions_between_all_chunks(&mut self) {
        let adjacent_region_connections = &self.split_detector.adjacent_region_connections;
        let uniform_chunk_adjacent_region_connections = &self
            .split_detector
            .uniform_chunk_adjacencent_region_connections;

        // The only regions whose connections with regions in other chunks need to be
        // resolved are the ones touching the chunk boundaries. We initialize each such
        // region for Disjoint Set Forest labeling by marking it as the representative
        // region in its own unique set by setting its parent label to have its own
        // region and chunk index (making it a root in the graph).
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
                            let chunk_regions = &mut self.split_detector.regions[start_region_idx
                                ..start_region_idx + chunk_data.boundary_region_count as usize];

                            for (region_idx, region) in chunk_regions.iter_mut().enumerate() {
                                region.parent_label =
                                    GlobalRegionLabel::new(chunk_idx as u32, region_idx as u32);
                            }
                        }
                        VoxelChunk::Uniform(chunk) => {
                            let region = &mut self.split_detector.regions
                                [chunk.split_detection.data_offset as usize];
                            let region_idx = 0;
                            region.parent_label =
                                GlobalRegionLabel::new(chunk_idx as u32, region_idx);
                        }
                        VoxelChunk::Empty => {}
                    }
                }
            }
        }

        // We then go through and join the roots of all local regions that are connected
        // based on the stored connection information
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
                            let chunk_boundary_region_range =
                                0..chunk_data.boundary_region_count as usize;

                            let chunk_adjacent_region_connections =
                                chunk_adjacent_region_connections(
                                    adjacent_region_connections,
                                    chunk.data_offset,
                                );

                            for region_idx in chunk_boundary_region_range {
                                let region =
                                    &self.split_detector.regions[start_region_idx + region_idx];
                                let parent_label = region.parent_label;

                                let range_of_adjacent_region_connections =
                                    region.range_of_adjacent_region_connections();

                                let region_root_label = if parent_label
                                    == GlobalRegionLabel::new(chunk_idx as u32, region_idx as u32)
                                {
                                    parent_label
                                } else {
                                    find_root_for_region_and_compress_path(
                                        &self.chunks,
                                        &mut self.split_detector.regions,
                                        self.split_detector.original_uniform_chunk_count,
                                        parent_label,
                                    )
                                };

                                for adjacent_region in &chunk_adjacent_region_connections
                                    [range_of_adjacent_region_connections]
                                {
                                    let adjacent_chunk_idx = adjacent_region
                                        .compute_relative_linear_chunk_idx(
                                            &self.chunk_idx_strides,
                                            chunk_idx,
                                        );
                                    let adjacent_region_idx = adjacent_region.region_idx() as usize;

                                    set_root_for_region(
                                        &self.chunks,
                                        &mut self.split_detector.regions,
                                        self.split_detector.original_uniform_chunk_count,
                                        adjacent_chunk_idx,
                                        adjacent_region_idx,
                                        region_root_label,
                                    );
                                }
                            }
                        }
                        VoxelChunk::Uniform(chunk) => {
                            let region = &self.split_detector.regions
                                [chunk.split_detection.data_offset as usize];
                            let parent_label = region.parent_label;

                            let range_of_adjacent_region_connections =
                                region.range_of_adjacent_region_connections();

                            let region_root_label =
                                if parent_label == GlobalRegionLabel::new(chunk_idx as u32, 0) {
                                    parent_label
                                } else {
                                    find_root_for_region_and_compress_path(
                                        &self.chunks,
                                        &mut self.split_detector.regions,
                                        self.split_detector.original_uniform_chunk_count,
                                        parent_label,
                                    )
                                };

                            let chunk_adjacent_region_connections =
                                chunk_adjacent_region_connections(
                                    uniform_chunk_adjacent_region_connections,
                                    chunk.split_detection.data_offset,
                                );

                            for adjacent_region in &chunk_adjacent_region_connections
                                [range_of_adjacent_region_connections]
                            {
                                let adjacent_chunk_idx = adjacent_region
                                    .compute_relative_linear_chunk_idx(
                                        &self.chunk_idx_strides,
                                        chunk_idx,
                                    );
                                let adjacent_region_idx = adjacent_region.region_idx() as usize;

                                set_root_for_region(
                                    &self.chunks,
                                    &mut self.split_detector.regions,
                                    self.split_detector.original_uniform_chunk_count,
                                    adjacent_chunk_idx,
                                    adjacent_region_idx,
                                    region_root_label,
                                );
                            }
                        }
                        VoxelChunk::Empty => {}
                    }
                }
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_region_count(&self) {
        let region_count = self.count_regions();
        assert!(region_count >= 1 || self.contains_only_empty_voxels());
        let expected_region_count = self.count_regions_brute_force();
        assert_eq!(region_count, expected_region_count);
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn count_regions_brute_force(&self) -> usize {
        let voxel_counts = self.chunk_counts.map(|count| count * CHUNK_SIZE);

        let mut parents = vec![0; voxel_counts.iter().product()];

        (0..parents.len()).for_each(|idx| {
            parents[idx] = idx;
        });

        let linear_idx =
            &|i, j, k| i * (voxel_counts[2] * voxel_counts[1]) + j * voxel_counts[2] + k;

        for i in self.occupied_voxel_ranges()[0].clone() {
            for j in self.occupied_voxel_ranges()[1].clone() {
                for k in self.occupied_voxel_ranges()[2].clone() {
                    if self.get_voxel(i, j, k).is_some() {
                        if i < voxel_counts[0] - 1 && self.get_voxel(i + 1, j, k).is_some() {
                            give_voxels_same_root_usize(
                                &mut parents,
                                linear_idx(i, j, k),
                                linear_idx(i + 1, j, k),
                            );
                        }
                        if j < voxel_counts[1] - 1 && self.get_voxel(i, j + 1, k).is_some() {
                            give_voxels_same_root_usize(
                                &mut parents,
                                linear_idx(i, j, k),
                                linear_idx(i, j + 1, k),
                            );
                        }
                        if k < voxel_counts[2] - 1 && self.get_voxel(i, j, k + 1).is_some() {
                            give_voxels_same_root_usize(
                                &mut parents,
                                linear_idx(i, j, k),
                                linear_idx(i, j, k + 1),
                            );
                        }
                    }
                }
            }
        }

        let mut region_count = 0;

        for i in self.occupied_voxel_ranges()[0].clone() {
            for j in self.occupied_voxel_ranges()[1].clone() {
                for k in self.occupied_voxel_ranges()[2].clone() {
                    if self.get_voxel(i, j, k).is_some() {
                        let idx = linear_idx(i, j, k);
                        if parents[idx] == idx {
                            region_count += 1;
                        }
                    }
                }
            }
        }

        region_count
    }
}

impl SplitDetector {
    /// Initializes the split detector, allocating buffers for the given number
    /// of uniform and non-uniform chunks.
    pub fn new(uniform_chunk_count: usize, non_uniform_chunk_count: usize) -> Self {
        let voxel_count = non_uniform_chunk_count << (3 * LOG2_CHUNK_SIZE);
        let voxel_region_label_count = voxel_count;
        let local_region_count = uniform_chunk_count + CHUNK_MAX_REGIONS * non_uniform_chunk_count;
        let adjacenct_region_connection_count =
            CHUNK_MAX_ADJACENT_REGION_CONNECTIONS * non_uniform_chunk_count;
        let uniform_chunk_adjacent_region_connection_count =
            CHUNK_MAX_ADJACENT_REGION_CONNECTIONS * uniform_chunk_count;
        Self {
            voxel_region_labels: vec![0; voxel_region_label_count],
            regions: vec![LocalRegion::zeroed(); local_region_count],
            original_uniform_chunk_count: uniform_chunk_count,
            adjacent_region_connections: vec![
                AdjacentRegionConnection::zero();
                adjacenct_region_connection_count
            ],
            uniform_chunk_adjacencent_region_connections: vec![
                AdjacentRegionConnection::zero();
                uniform_chunk_adjacent_region_connection_count
            ],
        }
    }

    /// Updates the buffers to account for the given uniform chunk having been
    /// converted to non-uniform.
    pub fn convert_uniform_chunk_to_non_uniform(
        &mut self,
        split_detection: UniformChunkSplitDetectionData,
    ) {
        // All voxels point to region zero, which is the representative region for the
        // entire chunk
        self.voxel_region_labels
            .extend(iter::repeat_n(0, CHUNK_VOXEL_COUNT));

        // Copy the chunk's local region from the uniform chunk section of the local
        // region buffer to the start of the new non-uniform chunk section at the end
        self.regions.reserve(CHUNK_MAX_REGIONS);
        self.regions
            .push(self.regions[split_detection.data_offset as usize]);
        self.regions
            .extend(iter::repeat_n(LocalRegion::zeroed(), CHUNK_MAX_REGIONS - 1));

        // Copy over the adjacent region connections from the uniform chunk buffer to
        // the non-uniform chunk buffer so that we don't invalidate the existing
        // adjacency information
        self.adjacent_region_connections
            .extend_from_slice(chunk_adjacent_region_connections(
                &self.uniform_chunk_adjacencent_region_connections,
                split_detection.data_offset,
            ));
    }

    /// Analyzes the given non-uniform chunk to determine all regions of
    /// connected voxels within the chunk. This should be called whenever any
    /// voxels in the chunk have changed state between present and empty.
    pub fn update_local_connected_regions_for_chunk(
        &mut self,
        voxels: &[Voxel],
        chunk: &mut NonUniformVoxelChunk,
        chunk_idx: u32,
    ) {
        let voxels = chunk_voxels(voxels, chunk.data_offset);

        let region_labels =
            chunk_voxel_region_labels_mut(&mut self.voxel_region_labels, chunk.data_offset);

        let regions = non_uniform_chunk_regions_mut(
            &mut self.regions,
            self.original_uniform_chunk_count,
            chunk.data_offset,
        );

        let chunk = &mut chunk.split_detection;

        // The entry at a given index contains the index to the parent entry, which is
        // an entry in the same local region as the child entry. If the entry is
        // its own parent, it is considered a root, and it acts a unique identifier for
        // the local region.
        let mut parents = [0; CHUNK_VOXEL_COUNT];

        // Start by making each voxel root (part of its own unique region) by assigning
        // its own index as its parent index
        (0..parents.len()).for_each(|idx| {
            parents[idx] = idx as u16;
        });

        // For each non-empty voxel, check which of its neighbors are also non-empty,
        // and make them part of the same local region if they are
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let idx = linear_voxel_idx_within_chunk(&[i, j, k]);
                    let voxel = voxels[idx];
                    if !voxel.is_empty() {
                        if i < CHUNK_SIZE - 1
                            && voxel.flags().contains(VoxelFlags::HAS_ADJACENT_X_UP)
                        {
                            give_voxels_same_root(
                                &mut parents,
                                idx,
                                linear_voxel_idx_within_chunk(&[i + 1, j, k]),
                            );
                        }
                        if j < CHUNK_SIZE - 1
                            && voxel.flags().contains(VoxelFlags::HAS_ADJACENT_Y_UP)
                        {
                            give_voxels_same_root(
                                &mut parents,
                                idx,
                                linear_voxel_idx_within_chunk(&[i, j + 1, k]),
                            );
                        }
                        if k < CHUNK_SIZE - 1
                            && voxel.flags().contains(VoxelFlags::HAS_ADJACENT_Z_UP)
                        {
                            give_voxels_same_root(
                                &mut parents,
                                idx,
                                linear_voxel_idx_within_chunk(&[i, j, k + 1]),
                            );
                        }
                    }
                }
            }
        }

        // We start by labeling representative voxels for regions along the faces,
        // making sure the representative voxel for each region connected to a
        // face is a face voxel

        let mut current_label = 0;

        // Since certain pathological voxel configurations could make us exceed the
        // limits we have put on the number of boundary regions, we have to put a
        // ceiling on the label. If we reach the limit, any additional regions
        // that are actually disconnected (at least within the chunk) from the
        // previous regions will be assigned to the same set as the last region.
        // This means we could technically miss disconnections, but for any
        // reasonable voxel configuration we will probably be fine.
        const MAX_BOUNDARY_LABEL: LocalRegionLabel =
            (CHUNK_MAX_BOUNDARY_REGIONS - 1) as LocalRegionLabel;

        #[allow(clippy::unnecessary_min_or_max)]
        for lp in LoopForChunkVoxels::over_full_boundary() {
            DataLoop3::new(&lp, voxels).execute(&mut |voxel_indices, voxel| {
                let idx = linear_voxel_idx_within_chunk(voxel_indices);

                if !voxel.is_empty() {
                    let set_id = find_root_for_voxel(&mut parents, idx);
                    if set_id == idx {
                        // If this is the representative voxel for its set, we give it a label
                        region_labels[idx] = current_label;
                        current_label = current_label.saturating_add(1).min(MAX_BOUNDARY_LABEL);
                    } else if chunk_voxel_indices_from_linear_idx(set_id)
                        .iter()
                        .all(|&index| index > 0 && index < CHUNK_SIZE - 1)
                    {
                        // If the representative voxel is in the interior, make this (face)
                        // voxel representative instead, and give it a label
                        make_voxel_root(&mut parents, idx, set_id);
                        region_labels[idx] = current_label;
                        current_label = current_label.saturating_add(1).min(MAX_BOUNDARY_LABEL);
                    } else {
                        // Otherwise, the representative voxel is on the
                        // chunk face, and has either already been labeled
                        // in a previous iteration or will be labeled in an
                        // upcoming iteration
                    }
                } else {
                    region_labels[idx] = EMPTY_VOXEL_LABEL;
                }
            });
        }

        assert!(current_label < MAX_BOUNDARY_LABEL);
        chunk.boundary_region_count = LocalRegionCount::from(current_label);

        // Label representative voxels for any internal-only regions

        const MAX_LABEL: LocalRegionLabel = (CHUNK_MAX_REGIONS - 1) as LocalRegionLabel;

        #[allow(clippy::unnecessary_min_or_max)]
        for i in 1..CHUNK_SIZE - 1 {
            for j in 1..CHUNK_SIZE - 1 {
                for k in 1..CHUNK_SIZE - 1 {
                    let idx = linear_voxel_idx_within_chunk(&[i, j, k]);
                    if parents[idx] as usize == idx {
                        if !voxels[idx].is_empty() {
                            region_labels[idx] = current_label;
                            current_label = current_label.saturating_add(1).min(MAX_LABEL);
                        } else {
                            region_labels[idx] = EMPTY_VOXEL_LABEL;
                        }
                    }
                }
            }
        }

        assert!(current_label < MAX_LABEL);
        chunk.region_count = LocalRegionCount::from(current_label);

        // Label all non-representative voxels with the label of their region

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let idx = linear_voxel_idx_within_chunk(&[i, j, k]);
                    if !voxels[idx].is_empty() {
                        let set_id = find_root_for_voxel(&mut parents, idx);
                        if set_id != idx {
                            region_labels[idx] = region_labels[set_id];
                        }
                        #[cfg(not(feature = "unchecked"))]
                        assert_ne!(region_labels[idx], EMPTY_VOXEL_LABEL);
                    }
                }
            }
        }

        // Old adjacent region connections are now invalidated
        for region in regions.iter_mut().take(chunk.region_count as usize) {
            region.adjacent_region_connection_start_idx = 0;
            region.adjacent_region_connection_count = 0;
        }

        let max_adjacent_region_connections_per_region =
            max_adjacent_region_connections_per_region(chunk.boundary_region_count);

        // Now that we know the number of boundary regions, we can give each of them a
        // range in the chunk's section of the adjacent region connection buffer. We
        // partition the available space evenly between the regions.
        for (region_idx, boundary_region) in regions
            .iter_mut()
            .take(chunk.boundary_region_count as usize)
            .enumerate()
        {
            boundary_region.adjacent_region_connection_start_idx =
                region_idx as LocalRegionCount * max_adjacent_region_connections_per_region;
        }

        // Since the interior regions can't be connected to regions in other chunks,
        // each of them is also a unique global region, and we can already mark them as
        // such by pointing their global region label to themselves. No need to involve
        // them in the global region resolution pass.
        for region_idx in chunk.boundary_region_count..chunk.region_count {
            regions[region_idx as usize].parent_label =
                GlobalRegionLabel::new(chunk_idx, u32::from(region_idx));
        }
    }

    fn copy_local_connected_regions_from_chunk_in_other(
        &mut self,
        this_chunk: &mut NonUniformVoxelChunk,
        this_chunk_idx: u32,
        other: &Self,
        other_chunk: &NonUniformVoxelChunk,
    ) {
        let these_region_labels =
            chunk_voxel_region_labels_mut(&mut self.voxel_region_labels, this_chunk.data_offset);
        let other_region_labels =
            chunk_voxel_region_labels(&other.voxel_region_labels, other_chunk.data_offset);
        these_region_labels.copy_from_slice(other_region_labels);

        let these_regions = non_uniform_chunk_regions_mut(
            &mut self.regions,
            self.original_uniform_chunk_count,
            this_chunk.data_offset,
        );
        let other_regions = non_uniform_chunk_regions(
            &other.regions,
            other.original_uniform_chunk_count,
            other_chunk.data_offset,
        );
        these_regions.copy_from_slice(other_regions);

        let this_chunk = &mut this_chunk.split_detection;
        let other_chunk = &other_chunk.split_detection;

        this_chunk.region_count = other_chunk.region_count;
        this_chunk.boundary_region_count = other_chunk.boundary_region_count;

        for region in these_regions
            .iter_mut()
            .take(this_chunk.region_count as usize)
        {
            region.adjacent_region_connection_start_idx = 0;
            region.adjacent_region_connection_count = 0;
        }

        for region_idx in this_chunk.boundary_region_count..this_chunk.region_count {
            these_regions[region_idx as usize].parent_label =
                GlobalRegionLabel::new(this_chunk_idx, u32::from(region_idx));
        }
    }

    /// Returns a [`NonUniformChunkConnectionUpdater`] that can be used to
    /// update the local region connections from the specified non-uniform
    /// chunk to the adjacent non-uniform chunk across the given face.
    #[must_use]
    pub fn begin_non_uniform_chunk_connection_update(
        &mut self,
        current_chunk_data_offset: u32,
        adjacent_chunk_data_offset: u32,
        current_chunk_split_detection: NonUniformChunkSplitDetectionData,
        face_dim: Dimension,
        face_side: Side,
    ) -> NonUniformChunkConnectionUpdater<'_> {
        NonUniformChunkConnectionUpdater::new(
            self,
            current_chunk_data_offset,
            adjacent_chunk_data_offset,
            current_chunk_split_detection.boundary_region_count,
            face_dim,
            face_side,
        )
    }

    /// Updates the connections between the single local regions in the given
    /// adjacent uniform chunks.
    pub fn update_mutual_connections_for_uniform_chunks(
        &mut self,
        lower_chunk_split_detection: UniformChunkSplitDetectionData,
        upper_chunk_split_detection: UniformChunkSplitDetectionData,
        face_dim: Dimension,
    ) {
        let (lower_chunk_region, upper_chunk_region) = uniform_regions_for_two_chunks_mut(
            &mut self.regions,
            lower_chunk_split_detection.data_offset,
            upper_chunk_split_detection.data_offset,
        );

        let (lower_chunk_adjacent_region_connections, upper_chunk_adjacent_region_connections) =
            adjacent_region_connections_for_two_chunks_mut(
                &mut self.uniform_chunk_adjacencent_region_connections,
                lower_chunk_split_detection.data_offset,
                upper_chunk_split_detection.data_offset,
            );

        remove_adjacent_connections_for_region(
            lower_chunk_region,
            lower_chunk_adjacent_region_connections,
            face_dim,
            Side::Upper,
        );

        remove_adjacent_connections_for_region(
            upper_chunk_region,
            upper_chunk_adjacent_region_connections,
            face_dim,
            Side::Lower,
        );

        add_adjacent_connection_for_region(
            lower_chunk_region,
            lower_chunk_adjacent_region_connections,
            CHUNK_MAX_ADJACENT_REGION_CONNECTIONS as LocalRegionCount,
            0,
            face_dim,
            Side::Upper,
        );

        add_adjacent_connection_for_region(
            upper_chunk_region,
            upper_chunk_adjacent_region_connections,
            CHUNK_MAX_ADJACENT_REGION_CONNECTIONS as LocalRegionCount,
            0,
            face_dim,
            Side::Lower,
        );
    }

    /// Removes all connections from the local regions in the given non-uniform
    /// chunk across the given face.
    pub fn remove_connections_for_non_uniform_chunk(
        &mut self,
        data_offset: u32,
        split_detection: NonUniformChunkSplitDetectionData,
        face_dim: Dimension,
        face_side: Side,
    ) {
        let chunk_regions = non_uniform_chunk_regions_mut(
            &mut self.regions,
            self.original_uniform_chunk_count,
            data_offset,
        );
        let chunk_adjacent_region_connections = chunk_adjacent_region_connections_mut(
            &mut self.adjacent_region_connections,
            data_offset,
        );

        remove_adjacent_connections_for_chunk_boundary_regions(
            chunk_regions,
            chunk_adjacent_region_connections,
            split_detection.boundary_region_count,
            face_dim,
            face_side,
        );
    }

    /// Updates the connections from the local regions in the given non-uniform
    /// chunk across the given face to the single local region in the adjacent
    /// uniform chunk.
    pub fn update_connections_from_non_uniform_chunk_to_uniform_chunk(
        &mut self,
        non_uniform_chunk_data_offset: u32,
        non_uniform_chunk_split_detection: NonUniformChunkSplitDetectionData,
        face_dim: Dimension,
        face_side: Side,
    ) {
        let non_uniform_chunk_regions = non_uniform_chunk_regions_mut(
            &mut self.regions,
            self.original_uniform_chunk_count,
            non_uniform_chunk_data_offset,
        );
        let non_uniform_chunk_adjacent_region_connections = chunk_adjacent_region_connections_mut(
            &mut self.adjacent_region_connections,
            non_uniform_chunk_data_offset,
        );

        // For each boundary region, remove all connections across this face, since we
        // are about to compute the new set of connections across the face
        remove_adjacent_connections_for_chunk_boundary_regions(
            non_uniform_chunk_regions,
            non_uniform_chunk_adjacent_region_connections,
            non_uniform_chunk_split_detection.boundary_region_count,
            face_dim,
            face_side,
        );

        let uniform_chunk_region_idx = 0;

        let non_uniform_chunk_voxel_region_labels = chunk_voxel_region_labels_mut(
            &mut self.voxel_region_labels,
            non_uniform_chunk_data_offset,
        );

        let max_adjacent_region_connections_per_region = max_adjacent_region_connections_per_region(
            non_uniform_chunk_split_detection.boundary_region_count,
        );

        // Since boundary regions are labeled first, the highest label we might
        // encounter when looking at boundary voxels is
        // `CHUNK_MAX_BOUNDARY_REGIONS - 1`
        let mut regions_connected = [0_u8; CHUNK_MAX_BOUNDARY_REGIONS];

        DataLoop3::new(
            &LoopForChunkVoxels::over_face(face_dim, face_side),
            non_uniform_chunk_voxel_region_labels,
        )
        .execute(&mut |_, &region_label| {
            if region_label == EMPTY_VOXEL_LABEL {
                return;
            }
            let connected = &mut regions_connected[region_label as usize];
            if *connected == 0 {
                *connected = 1;

                non_uniform_chunk_adjacent_region_connections[non_uniform_chunk_regions
                    [region_label as usize]
                    .push_adjacent_region_connection_idx(
                        max_adjacent_region_connections_per_region,
                    )] =
                    AdjacentRegionConnection::new(uniform_chunk_region_idx, face_dim, face_side);
            }
        });
    }

    /// Updates the connections from the single local region in the given
    /// uniform chunk across the given face to the local regions in the
    /// adjacent non-uniform chunk.
    pub fn update_connections_from_uniform_chunk_to_non_uniform_chunk(
        &mut self,
        uniform_chunk_split_detection: UniformChunkSplitDetectionData,
        non_uniform_chunk_data_offset: u32,
        face_dim: Dimension,
        face_side: Side,
    ) {
        let uniform_chunk_region =
            &mut self.regions[uniform_chunk_split_detection.data_offset as usize];

        let uniform_chunk_adjacent_region_connections = chunk_adjacent_region_connections_mut(
            &mut self.uniform_chunk_adjacencent_region_connections,
            uniform_chunk_split_detection.data_offset,
        );

        remove_adjacent_connections_for_region(
            uniform_chunk_region,
            uniform_chunk_adjacent_region_connections,
            face_dim,
            face_side,
        );

        let non_uniform_chunk_voxel_region_labels = chunk_voxel_region_labels_mut(
            &mut self.voxel_region_labels,
            non_uniform_chunk_data_offset,
        );

        let mut regions_connected = [0_u8; CHUNK_MAX_BOUNDARY_REGIONS];

        DataLoop3::new(
            &LoopForChunkVoxels::over_face(face_dim, face_side.opposite()),
            non_uniform_chunk_voxel_region_labels,
        )
        .execute(&mut |_, &region_label| {
            if region_label == EMPTY_VOXEL_LABEL {
                return;
            }
            let connected = &mut regions_connected[region_label as usize];
            if *connected == 0 {
                *connected = 1;

                uniform_chunk_adjacent_region_connections[uniform_chunk_region
                    .push_adjacent_region_connection_idx(
                        CHUNK_MAX_ADJACENT_REGION_CONNECTIONS as LocalRegionCount,
                    )] =
                    AdjacentRegionConnection::new(u16::from(region_label), face_dim, face_side);
            }
        });
    }

    /// Updates the connections from the local regions in the given non-uniform
    /// chunk across the given face to the local regions in the adjacent
    /// non-uniform chunk, assuming that the adjoining face of the adjacent
    /// chunk is completely filled by voxels (this is more efficient than using
    /// the more general [`NonUniformChunkConnectionUpdater`]).
    pub fn update_connections_from_non_uniform_chunk_to_non_uniform_chunk_with_full_face(
        &mut self,
        current_chunk_data_offset: u32,
        current_chunk_split_detection: NonUniformChunkSplitDetectionData,
        adjacent_chunk_data_offset: u32,
        face_dim: Dimension,
        face_side: Side,
    ) {
        let current_chunk_regions = non_uniform_chunk_regions_mut(
            &mut self.regions,
            self.original_uniform_chunk_count,
            current_chunk_data_offset,
        );
        let current_chunk_adjacent_region_connections = chunk_adjacent_region_connections_mut(
            &mut self.adjacent_region_connections,
            current_chunk_data_offset,
        );

        // For each boundary region, remove all connections across this face, since we
        // are about to compute the new set of connections across the face
        remove_adjacent_connections_for_chunk_boundary_regions(
            current_chunk_regions,
            current_chunk_adjacent_region_connections,
            current_chunk_split_detection.boundary_region_count,
            face_dim,
            face_side,
        );

        let adjacent_chunk_voxel_region_labels = chunk_voxel_region_labels_mut(
            &mut self.voxel_region_labels,
            adjacent_chunk_data_offset,
        );

        let mut adjacent_face_voxel_indices = [0; 3];
        if face_side == Side::Lower {
            adjacent_face_voxel_indices[face_dim.idx()] = CHUNK_SIZE - 1;
        }
        let adjacent_voxel_region_label = adjacent_chunk_voxel_region_labels
            [linear_voxel_idx_within_chunk(&adjacent_face_voxel_indices)];
        assert_ne!(adjacent_voxel_region_label, EMPTY_VOXEL_LABEL);

        let current_chunk_voxel_region_labels =
            chunk_voxel_region_labels_mut(&mut self.voxel_region_labels, current_chunk_data_offset);

        let max_adjacent_region_connections_per_region = max_adjacent_region_connections_per_region(
            current_chunk_split_detection.boundary_region_count,
        );

        let mut regions_connected = [0_u8; CHUNK_MAX_BOUNDARY_REGIONS];

        DataLoop3::new(
            &LoopForChunkVoxels::over_face(face_dim, face_side),
            current_chunk_voxel_region_labels,
        )
        .execute(&mut |_, &region_label| {
            if region_label == EMPTY_VOXEL_LABEL {
                return;
            }
            let connected = &mut regions_connected[region_label as usize];
            if *connected == 0 {
                *connected = 1;

                current_chunk_adjacent_region_connections[current_chunk_regions
                    [region_label as usize]
                    .push_adjacent_region_connection_idx(
                        max_adjacent_region_connections_per_region,
                    )] = AdjacentRegionConnection::new(
                    u16::from(adjacent_voxel_region_label),
                    face_dim,
                    face_side,
                );
            }
        });
    }
}

impl<'a> NonUniformChunkConnectionUpdater<'a> {
    fn new(
        split_detector: &'a mut SplitDetector,
        current_chunk_data_offset: u32,
        adjacent_chunk_data_offset: u32,
        current_chunk_boundary_region_count: LocalRegionCount,
        face_dim: Dimension,
        face_side: Side,
    ) -> Self {
        let (current_chunk_voxel_region_labels, adjacent_chunk_voxel_region_labels) =
            voxel_region_labels_for_two_chunks_mut(
                &mut split_detector.voxel_region_labels,
                current_chunk_data_offset,
                adjacent_chunk_data_offset,
            );

        let current_chunk_regions = non_uniform_chunk_regions_mut(
            &mut split_detector.regions,
            split_detector.original_uniform_chunk_count,
            current_chunk_data_offset,
        );

        let current_chunk_adjacent_region_connections = chunk_adjacent_region_connections_mut(
            &mut split_detector.adjacent_region_connections,
            current_chunk_data_offset,
        );

        // For each boundary region, remove all connections across this face, since we
        // are about to compute the new set of connections across the face
        remove_adjacent_connections_for_chunk_boundary_regions(
            current_chunk_regions,
            current_chunk_adjacent_region_connections,
            current_chunk_boundary_region_count,
            face_dim,
            face_side,
        );

        let max_adjacent_region_connections_per_region =
            max_adjacent_region_connections_per_region(current_chunk_boundary_region_count);

        // We need to keep track of all pairs of local regions that we have added a
        // connection for so that we don't duplicate connections. For any of the up to
        // `CHUNK_MAX_BOUNDARY_REGIONS` local regions in this chunk, there could be a
        // connection with any of the up to `CHUNK_MAX_BOUNDARY_REGIONS` local regions
        // in the adjacent chunk. Since each entry in this 2D map of connections is a
        // boolean value, we can improve cache locality by packing 8 values into a byte.
        let regions_connected = [[0; CHUNK_MAX_BOUNDARY_REGIONS / 8]; CHUNK_MAX_BOUNDARY_REGIONS];

        Self {
            current_chunk_voxel_region_labels,
            adjacent_chunk_voxel_region_labels,
            current_chunk_regions,
            current_chunk_adjacent_region_connections,
            max_adjacent_region_connections_per_region,
            face_dim,
            face_side,
            regions_connected,
        }
    }

    /// Updates the connection from the local region containing the given
    /// non-empty voxel in the current chunk to the local region containing the
    /// given non-empty voxel in the adjacent chunk.
    pub fn update_for_non_empty_adjacent_voxel(
        &mut self,
        current_chunk_voxel_idx: usize,
        adjacent_chunk_voxel_idx: usize,
    ) {
        let current_voxel_region_label =
            self.current_chunk_voxel_region_labels[current_chunk_voxel_idx] as usize;
        let adjacent_voxel_region_label =
            self.adjacent_chunk_voxel_region_labels[adjacent_chunk_voxel_idx];

        // Obtain the byte containing the bit indicating whether a connection already
        // exists for the two regions
        let connected_bitfield = &mut self.regions_connected[current_voxel_region_label]
            [(adjacent_voxel_region_label >> 3) as usize];

        // Isolate the relevant bit, whose position is determined by the value in the
        // lower three bits in the label (the ones shifted out above)
        let bit_mask = 1 << (adjacent_voxel_region_label & 0b111);

        if *connected_bitfield & bit_mask == 0 {
            *connected_bitfield |= bit_mask;

            let connection = AdjacentRegionConnection::new(
                u16::from(adjacent_voxel_region_label),
                self.face_dim,
                self.face_side,
            );

            let current_region = &mut self.current_chunk_regions[current_voxel_region_label];

            self.current_chunk_adjacent_region_connections[current_region
                .push_adjacent_region_connection_idx(
                    self.max_adjacent_region_connections_per_region,
                )] = connection;
        }
    }
}

impl UniformChunkSplitDetectionData {
    /// Creates split detection data for a new uniform chunk given the previous
    /// number of uniform chunks, that is, the count excluding this chunk.
    #[inline]
    pub fn new(previous_uniform_chunk_count: usize) -> Self {
        Self {
            data_offset: previous_uniform_chunk_count as u32,
        }
    }
}

impl NonUniformChunkSplitDetectionData {
    /// Initializes split detection data for a new non-uniform chunk.
    #[inline]
    pub fn new() -> Self {
        Self {
            region_count: 0,
            boundary_region_count: 0,
        }
    }

    /// Creates split detection data for a chunk that was just converted from
    /// uniform to non-uniform.
    #[inline]
    pub fn for_previously_uniform() -> Self {
        Self {
            region_count: 1,
            boundary_region_count: 1,
        }
    }
}

impl Default for NonUniformChunkSplitDetectionData {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl LocalRegion {
    #[inline]
    const fn zeroed() -> Self {
        Self {
            parent_label: GlobalRegionLabel::zero(),
            adjacent_region_connection_start_idx: 0,
            adjacent_region_connection_count: 0,
        }
    }

    #[inline]
    const fn range_of_adjacent_region_connections(&self) -> Range<usize> {
        (self.adjacent_region_connection_start_idx as usize)
            ..(self.adjacent_region_connection_start_idx as usize
                + self.adjacent_region_connection_count as usize)
    }

    #[inline]
    fn push_adjacent_region_connection_idx(
        &mut self,
        max_adjacent_region_connections: LocalRegionCount,
    ) -> usize {
        // We need to make sure we don't exceed the maximum number of adjacent region
        // connection slots allocated to each local region in the chunk
        if self.adjacent_region_connection_count < max_adjacent_region_connections {
            let new_idx = self.adjacent_region_connection_start_idx as usize
                + self.adjacent_region_connection_count as usize;
            self.adjacent_region_connection_count += 1;
            new_idx
        } else {
            // If we would exceed the limit, we instead overwrite the entry in the last slot
            impact_log::warn!("Exceeded max supported number of adjacent regions");
            self.adjacent_region_connection_start_idx as usize
                + (self.adjacent_region_connection_count - 1) as usize
        }
    }
}

impl GlobalRegionLabel {
    const CHUNK_IDX_N_BITS: u32 = 32 - Self::REGION_IDX_N_BITS;
    const REGION_IDX_N_BITS: u32 = 8;
    const CHUNK_IDX_MASK: u32 = (1 << Self::CHUNK_IDX_N_BITS) - 1;
    const REGION_IDX_MASK: u32 = (1 << Self::REGION_IDX_N_BITS) - 1;

    #[inline]
    const fn zero() -> Self {
        Self(0)
    }

    #[inline]
    fn new(chunk_idx: u32, region_idx: u32) -> Self {
        assert!(chunk_idx <= Self::CHUNK_IDX_MASK);
        assert!(region_idx <= Self::REGION_IDX_MASK);

        let mut bits = 0;
        bits |= (chunk_idx & Self::CHUNK_IDX_MASK) << Self::REGION_IDX_N_BITS;
        bits |= region_idx & Self::REGION_IDX_MASK;

        Self(bits)
    }

    #[inline]
    fn chunk_idx(&self) -> u32 {
        (self.0 >> Self::REGION_IDX_N_BITS) & Self::CHUNK_IDX_MASK
    }

    #[inline]
    fn region_idx(&self) -> u32 {
        self.0 & Self::REGION_IDX_MASK
    }
}

impl AdjacentRegionConnection {
    const REGION_IDX_N_BITS: u16 = 12;
    const CHUNK_FACE_N_BITS: u16 = 4;
    const REGION_IDX_MASK: u16 = (1 << Self::REGION_IDX_N_BITS) - 1;
    const REGION_IDX_SHIFT: u16 = Self::CHUNK_FACE_N_BITS;

    #[inline]
    const fn zero() -> Self {
        Self(0)
    }

    #[inline]
    fn new(region_idx: u16, face_dim: Dimension, face_side: Side) -> Self {
        assert!(region_idx <= Self::REGION_IDX_MASK);

        let mut bits = 0;
        bits |= (region_idx & Self::REGION_IDX_MASK) << Self::REGION_IDX_SHIFT;
        bits |= Self::encode_face(face_dim, face_side);

        Self(bits)
    }

    #[inline]
    fn encode_face(face_dim: Dimension, face_side: Side) -> u16 {
        let encoded_dim = match face_dim {
            Dimension::X => 0b1000,
            Dimension::Y => 0b0100,
            Dimension::Z => 0b0010,
        };
        let encoded_side = match face_side {
            Side::Lower => 0b0000,
            Side::Upper => 0b0001,
        };
        encoded_dim | encoded_side
    }

    #[inline]
    fn decode_face(&self) -> (Dimension, Side) {
        let face_dim = match self.0 & 0b1110 {
            0b1000 => Dimension::X,
            0b0100 => Dimension::Y,
            0b0010 => Dimension::Z,
            _ => unreachable!(),
        };
        let face_side = if (self.0 & 0b0001) == 0 {
            Side::Lower
        } else {
            Side::Upper
        };
        (face_dim, face_side)
    }

    #[inline]
    fn region_idx(&self) -> u16 {
        (self.0 >> Self::REGION_IDX_SHIFT) & Self::REGION_IDX_MASK
    }

    #[inline]
    fn compute_relative_linear_chunk_idx(
        &self,
        chunk_idx_strides: &[usize; 3],
        mut chunk_idx: usize,
    ) -> usize {
        let (face_dim, face_side) = self.decode_face();
        let stride = chunk_idx_strides[face_dim.idx()];
        match face_side {
            Side::Lower => {
                assert_ne!(chunk_idx, 0);
                chunk_idx -= stride;
            }
            Side::Upper => {
                chunk_idx += stride;
            }
        }
        chunk_idx
    }
}

impl PropertyTransferrer for NoPropertyTransferrer {
    fn transfer_voxel(&mut self, _object_voxel_indices: &[usize; 3], _voxel: Voxel) {}

    fn transfer_non_uniform_chunk(&mut self, _chunk_indices: &[usize; 3], _chunk_voxels: &[Voxel]) {
    }

    fn transfer_uniform_chunk(&mut self, _chunk_indices: &[usize; 3], _chunk_voxel: Voxel) {}
}

#[inline]
const fn non_uniform_chunk_start_region_idx(uniform_chunk_count: usize, data_offset: u32) -> usize {
    uniform_chunk_count + ((data_offset as usize) << LOG2_MAX_REGIONS_PER_CHUNK)
}

#[inline]
const fn chunk_start_adjacent_region_idx(data_offset: u32) -> usize {
    (data_offset as usize) * CHUNK_MAX_ADJACENT_REGION_CONNECTIONS
}

#[inline]
fn chunk_voxel_region_labels(
    voxel_region_labels: &[LocalRegionLabel],
    data_offset: u32,
) -> &[LocalRegionLabel] {
    let start_voxel_idx = chunk_start_voxel_idx(data_offset);
    &voxel_region_labels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
}

#[inline]
fn chunk_voxel_region_labels_mut(
    voxel_region_labels: &mut [LocalRegionLabel],
    data_offset: u32,
) -> &mut [LocalRegionLabel] {
    let start_voxel_idx = chunk_start_voxel_idx(data_offset);
    &mut voxel_region_labels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
}

#[inline]
fn non_uniform_chunk_regions(
    regions: &[LocalRegion],
    uniform_chunk_count: usize,
    data_offset: u32,
) -> &[LocalRegion] {
    let start_region_idx = non_uniform_chunk_start_region_idx(uniform_chunk_count, data_offset);
    &regions[start_region_idx..start_region_idx + CHUNK_MAX_REGIONS]
}

#[inline]
fn non_uniform_chunk_regions_mut(
    regions: &mut [LocalRegion],
    uniform_chunk_count: usize,
    data_offset: u32,
) -> &mut [LocalRegion] {
    let start_region_idx = non_uniform_chunk_start_region_idx(uniform_chunk_count, data_offset);
    &mut regions[start_region_idx..start_region_idx + CHUNK_MAX_REGIONS]
}

#[inline]
fn chunk_adjacent_region_connections(
    adjacent_region_connections: &[AdjacentRegionConnection],
    data_offset: u32,
) -> &[AdjacentRegionConnection] {
    let start_adjacent_region_idx = chunk_start_adjacent_region_idx(data_offset);
    &adjacent_region_connections[start_adjacent_region_idx
        ..start_adjacent_region_idx + CHUNK_MAX_ADJACENT_REGION_CONNECTIONS]
}

#[inline]
fn chunk_adjacent_region_connections_mut(
    adjacent_region_connections: &mut [AdjacentRegionConnection],
    data_offset: u32,
) -> &mut [AdjacentRegionConnection] {
    let start_adjacent_region_idx = chunk_start_adjacent_region_idx(data_offset);
    &mut adjacent_region_connections[start_adjacent_region_idx
        ..start_adjacent_region_idx + CHUNK_MAX_ADJACENT_REGION_CONNECTIONS]
}

#[inline]
fn voxel_region_labels_for_two_chunks_mut(
    voxel_region_labels: &mut [LocalRegionLabel],
    chunk_1_data_offset: u32,
    chunk_2_data_offset: u32,
) -> (&mut [LocalRegionLabel], &mut [LocalRegionLabel]) {
    extract_slice_segments_mut(
        voxel_region_labels,
        chunk_start_voxel_idx(chunk_1_data_offset),
        chunk_start_voxel_idx(chunk_2_data_offset),
        CHUNK_VOXEL_COUNT,
    )
}

#[inline]
fn uniform_regions_for_two_chunks_mut(
    regions: &mut [LocalRegion],
    chunk_1_data_offset: u32,
    chunk_2_data_offset: u32,
) -> (&mut LocalRegion, &mut LocalRegion) {
    let (chunk_1_regions, chunk_2_regions) = extract_slice_segments_mut(
        regions,
        chunk_1_data_offset as usize,
        chunk_2_data_offset as usize,
        1,
    );
    (&mut chunk_1_regions[0], &mut chunk_2_regions[0])
}

#[inline]
fn adjacent_region_connections_for_two_chunks_mut(
    adjacent_region_connections: &mut [AdjacentRegionConnection],
    chunk_1_data_offset: u32,
    chunk_2_data_offset: u32,
) -> (
    &mut [AdjacentRegionConnection],
    &mut [AdjacentRegionConnection],
) {
    extract_slice_segments_mut(
        adjacent_region_connections,
        chunk_start_adjacent_region_idx(chunk_1_data_offset),
        chunk_start_adjacent_region_idx(chunk_2_data_offset),
        CHUNK_MAX_ADJACENT_REGION_CONNECTIONS,
    )
}

#[inline]
fn max_adjacent_region_connections_per_region(
    boundary_region_count: LocalRegionCount,
) -> LocalRegionCount {
    CHUNK_MAX_ADJACENT_REGION_CONNECTIONS as LocalRegionCount / boundary_region_count.max(1)
}

/// Walks the trees of parent voxels for the two voxels with the given indices
/// to find their roots and merges the trees by assigning one root as the parent
/// of the other.
#[inline]
fn give_voxels_same_root(parents: &mut [u16], idx_1: usize, idx_2: usize) {
    let root_1_idx = find_root_for_voxel(parents, idx_1);
    let root_2_idx = find_root_for_voxel(parents, idx_2);

    if root_1_idx != root_2_idx {
        // WARNING: Changing the order of these is extremely damaging for performance.
        // There is most likely only one region in the chunk, and from the way we
        // iterate over the voxels, and most calls to this function will result in the
        // same `root_1_idx` (which is most likely the first voxel) but a different
        // `root_2_idx` (which is most likely the current adjacent voxel).
        // Jumping back to update the root of the first voxel every time is much
        // worse for cache locality than updating the root of the adjacent
        // voxel.
        cfg_if! {
            if #[cfg(feature = "unchecked")] {
                unsafe{ *parents.get_unchecked_mut(root_2_idx) = root_1_idx as u16; }
            } else {
                parents[root_2_idx] = root_1_idx as u16;
            }
        }
    }
}

/// Walks the tree of parent voxels for the voxel with the given index until
/// the root voxel index representing the local region is found.
#[cfg(not(feature = "unchecked"))]
#[inline]
fn find_root_for_voxel(parents: &mut [u16], idx: usize) -> usize {
    let parent_idx = parents[idx] as usize;

    // If the parent is the same entry, this is a root
    if parent_idx == idx {
        return parent_idx;
    }

    let root_idx = find_root_for_voxel(parents, parent_idx);

    // Compress the path to the root by making the root the direct parent
    parents[idx] = root_idx as u16;

    root_idx
}

/// Walks the tree of parent voxels for the voxel with the given index until
/// the root voxel index representing the local region is found.
#[cfg(feature = "unchecked")]
#[inline]
fn find_root_for_voxel(parents: &mut [u16], idx: usize) -> usize {
    let parent_idx = unsafe { *parents.get_unchecked(idx) as usize };

    // If the parent is the same entry, this is a root
    if parent_idx == idx {
        return parent_idx;
    }

    // Unroll the recursion once to reduce call overhead
    let root_idx = {
        let grandparent_idx = unsafe { *parents.get_unchecked(parent_idx) as usize };
        if grandparent_idx == parent_idx {
            grandparent_idx
        } else {
            find_root_for_voxel_alternating_compression(parents, grandparent_idx)
            // <- Skipping path compression at this point tends to be faster
        }
    };

    // Compress the path to the root by making the root the direct parent
    unsafe {
        *parents.get_unchecked_mut(idx) = root_idx as u16;
    }

    root_idx
}

#[cfg(feature = "unchecked")]
fn find_root_for_voxel_alternating_compression(parents: &mut [u16], idx: usize) -> usize {
    let parent_idx = unsafe { *parents.get_unchecked(idx) as usize };

    if parent_idx == idx {
        return parent_idx;
    }

    let grandparent_idx = unsafe { *parents.get_unchecked(parent_idx) as usize };

    if grandparent_idx == parent_idx {
        return grandparent_idx;
    }

    let root_idx = find_root_for_voxel_alternating_compression(parents, grandparent_idx);

    unsafe {
        *parents.get_unchecked_mut(parent_idx) = root_idx as u16;
    }

    root_idx
}

#[cfg(not(feature = "unchecked"))]
#[inline]
fn make_voxel_root(parents: &mut [u16], idx: usize, root_idx: usize) {
    // Set the new root voxel as the parent of the old root voxel
    parents[root_idx] = idx as u16;
    // Make the new root voxel its own parent, marking it as a root
    parents[idx] = idx as u16;
}

#[cfg(feature = "unchecked")]
#[inline]
fn make_voxel_root(parents: &mut [u16], idx: usize, root_idx: usize) {
    unsafe {
        // Set the new root voxel as the parent of the old root voxel
        *parents.get_unchecked_mut(root_idx) = idx as u16;
        // Make the new root voxel its own parent, marking it as a root
        *parents.get_unchecked_mut(idx) = idx as u16;
    }
}

#[cfg(any(test, feature = "fuzzing"))]
#[inline]
fn give_voxels_same_root_usize(parents: &mut [usize], idx_1: usize, idx_2: usize) {
    let root_1_idx = find_root_for_voxel_usize(parents, idx_1);
    let root_2_idx = find_root_for_voxel_usize(parents, idx_2);

    if root_1_idx != root_2_idx {
        parents[root_2_idx] = root_1_idx;
    }
}

#[cfg(any(test, feature = "fuzzing"))]
fn find_root_for_voxel_usize(parents: &mut [usize], idx: usize) -> usize {
    let parent_idx = parents[idx];

    if parent_idx == idx {
        return parent_idx;
    }

    let root_idx = find_root_for_voxel_usize(parents, parent_idx);

    parents[idx] = root_idx;

    root_idx
}

/// Walks the tree of parent regions for the given region to find its
/// root and merges the tree with the tree with the given target root by
/// assigning the target root as the parent of the current root.
#[cfg(not(feature = "unchecked"))]
#[inline]
fn set_root_for_region(
    chunks: &[VoxelChunk],
    regions: &mut [LocalRegion],
    uniform_chunk_count: usize,
    region_chunk_idx: usize,
    region_idx: usize,
    root_label: GlobalRegionLabel,
) {
    let global_region_idx = object_region_idx_for_region_in_chunk(
        chunks,
        uniform_chunk_count,
        region_chunk_idx,
        region_idx,
    );
    let region = &regions[global_region_idx];

    let region_root_label = find_root_for_region_and_compress_path(
        chunks,
        regions,
        uniform_chunk_count,
        region.parent_label,
    );

    if region_root_label != root_label {
        let global_root_region_idx =
            object_region_idx_for_region_label(chunks, uniform_chunk_count, region_root_label);
        regions[global_root_region_idx].parent_label = root_label;
    }
}

#[cfg(feature = "unchecked")]
#[inline]
fn set_root_for_region(
    chunks: &[VoxelChunk],
    regions: &mut [LocalRegion],
    uniform_chunk_count: usize,
    region_chunk_idx: usize,
    region_idx: usize,
    root_label: GlobalRegionLabel,
) {
    let global_region_idx = object_region_idx_for_region_in_chunk(
        chunks,
        uniform_chunk_count,
        region_chunk_idx,
        region_idx,
    );
    let region = unsafe { regions.get_unchecked(global_region_idx) };

    let region_root_label = find_root_for_region_and_compress_path(
        chunks,
        regions,
        uniform_chunk_count,
        region.parent_label,
    );

    if region_root_label != root_label {
        let global_root_region_idx =
            object_region_idx_for_region_label(chunks, uniform_chunk_count, region_root_label);
        unsafe {
            regions
                .get_unchecked_mut(global_root_region_idx)
                .parent_label = root_label;
        }
    }
}

/// Walks the tree of parent regions for the region with the given label until
/// the root region label identifying the global region is found. The path to
/// the root is then shortened by making the root the direct parent of the
/// region.
#[cfg(not(feature = "unchecked"))]
fn find_root_for_region_and_compress_path(
    chunks: &[VoxelChunk],
    regions: &mut [LocalRegion],
    uniform_chunk_count: usize,
    region_label: GlobalRegionLabel,
) -> GlobalRegionLabel {
    let region_idx = object_region_idx_for_region_label(chunks, uniform_chunk_count, region_label);
    let region = regions[region_idx];

    // If the parent is the same region, this is a root
    if region.parent_label == region_label {
        return region_label;
    }

    let region_root_label = find_root_for_region_and_compress_path(
        chunks,
        regions,
        uniform_chunk_count,
        region.parent_label,
    );

    // Compress the path to the root by making the root the direct parent
    regions[region_idx].parent_label = region_root_label;

    region_root_label
}

/// Walks the tree of parent regions for the region with the given label until
/// the root region label identifying the global region is found. The path to
/// the root is then shortened by making the root the direct parent of the
/// region.
#[cfg(feature = "unchecked")]
#[inline]
fn find_root_for_region_and_compress_path(
    chunks: &[VoxelChunk],
    regions: &mut [LocalRegion],
    uniform_chunk_count: usize,
    region_label: GlobalRegionLabel,
) -> GlobalRegionLabel {
    let region_idx = object_region_idx_for_region_label(chunks, uniform_chunk_count, region_label);
    let region = unsafe { regions.get_unchecked(region_idx) };

    if region.parent_label == region_label {
        return region_label;
    }

    let region_root_label = {
        let parent_region_idx =
            object_region_idx_for_region_label(chunks, uniform_chunk_count, region.parent_label);
        let parent_region = unsafe { regions.get_unchecked(parent_region_idx) };

        if parent_region.parent_label == region.parent_label {
            region.parent_label
        } else {
            find_root_for_region_alternating_compression(
                chunks,
                regions,
                uniform_chunk_count,
                parent_region.parent_label,
            )
        }
    };

    unsafe {
        regions.get_unchecked_mut(region_idx).parent_label = region_root_label;
    }

    region_root_label
}

#[cfg(feature = "unchecked")]
fn find_root_for_region_alternating_compression(
    chunks: &[VoxelChunk],
    regions: &mut [LocalRegion],
    uniform_chunk_count: usize,
    region_label: GlobalRegionLabel,
) -> GlobalRegionLabel {
    let region_idx = object_region_idx_for_region_label(chunks, uniform_chunk_count, region_label);
    let region = unsafe { regions.get_unchecked(region_idx) };

    if region.parent_label == region_label {
        return region_label;
    }

    let parent_region_idx =
        object_region_idx_for_region_label(chunks, uniform_chunk_count, region.parent_label);
    let parent_region = unsafe { regions.get_unchecked(parent_region_idx) };

    if parent_region.parent_label == region.parent_label {
        return region.parent_label;
    }

    let region_root_label = find_root_for_region_alternating_compression(
        chunks,
        regions,
        uniform_chunk_count,
        parent_region.parent_label,
    );

    unsafe {
        regions.get_unchecked_mut(parent_region_idx).parent_label = region_root_label;
    }

    region_root_label
}

fn find_root_for_region(
    chunks: &[VoxelChunk],
    regions: &[LocalRegion],
    uniform_chunk_count: usize,
    region_label: GlobalRegionLabel,
) -> GlobalRegionLabel {
    let region_idx = object_region_idx_for_region_label(chunks, uniform_chunk_count, region_label);
    let region = regions[region_idx];

    if region.parent_label == region_label {
        return region_label;
    }

    let region_root_label = {
        let parent_region_idx =
            object_region_idx_for_region_label(chunks, uniform_chunk_count, region.parent_label);
        let parent_region = &regions[parent_region_idx];

        if parent_region.parent_label == region.parent_label {
            region.parent_label
        } else {
            find_root_for_region(
                chunks,
                regions,
                uniform_chunk_count,
                parent_region.parent_label,
            )
        }
    };

    region_root_label
}

#[inline]
fn object_region_idx_for_region_label(
    chunks: &[VoxelChunk],
    uniform_chunk_count: usize,
    region_label: GlobalRegionLabel,
) -> usize {
    object_region_idx_for_region_in_chunk(
        chunks,
        uniform_chunk_count,
        region_label.chunk_idx() as usize,
        region_label.region_idx() as usize,
    )
}

#[cfg(not(feature = "unchecked"))]
#[inline]
fn object_region_idx_for_region_in_chunk(
    chunks: &[VoxelChunk],
    uniform_chunk_count: usize,
    chunk_idx: usize,
    region_idx: usize,
) -> usize {
    match &chunks[chunk_idx] {
        VoxelChunk::NonUniform(NonUniformVoxelChunk { data_offset, .. }) => {
            non_uniform_chunk_start_region_idx(uniform_chunk_count, *data_offset) + region_idx
        }
        VoxelChunk::Uniform(UniformVoxelChunk {
            split_detection: UniformChunkSplitDetectionData { data_offset },
            ..
        }) => {
            assert_eq!(region_idx, 0);
            *data_offset as usize
        }
        VoxelChunk::Empty => panic!("Got empty chunk in connected region resolution"),
    }
}

#[cfg(feature = "unchecked")]
#[inline]
fn object_region_idx_for_region_in_chunk(
    chunks: &[VoxelChunk],
    uniform_chunk_count: usize,
    chunk_idx: usize,
    region_idx: usize,
) -> usize {
    match unsafe { chunks.get_unchecked(chunk_idx) } {
        VoxelChunk::NonUniform(NonUniformVoxelChunk { data_offset, .. }) => {
            non_uniform_chunk_start_region_idx(uniform_chunk_count, *data_offset) + region_idx
        }
        VoxelChunk::Uniform(UniformVoxelChunk {
            split_detection: UniformChunkSplitDetectionData { data_offset },
            ..
        }) => *data_offset as usize,
        VoxelChunk::Empty => unsafe { std::hint::unreachable_unchecked() },
    }
}

#[inline]
fn remove_adjacent_connections_for_chunk_boundary_regions(
    chunk_regions: &mut [LocalRegion],
    chunk_adjacent_region_connections: &mut [AdjacentRegionConnection],
    chunk_boundary_region_count: LocalRegionCount,
    face_dim: Dimension,
    face_side: Side,
) {
    for boundary_region in chunk_regions
        .iter_mut()
        .take(chunk_boundary_region_count as usize)
    {
        remove_adjacent_connections_for_region(
            boundary_region,
            chunk_adjacent_region_connections,
            face_dim,
            face_side,
        );
    }
}

#[inline]
fn remove_adjacent_connections_for_region(
    region: &mut LocalRegion,
    chunk_adjacent_region_connections: &mut [AdjacentRegionConnection],
    face_dim: Dimension,
    face_side: Side,
) {
    let adjacent_region_connections =
        &mut chunk_adjacent_region_connections[region.range_of_adjacent_region_connections()];

    let mut idx = 0;
    while idx < region.adjacent_region_connection_count as usize {
        if adjacent_region_connections[idx].decode_face() == (face_dim, face_side) {
            region.adjacent_region_connection_count -= 1;
            adjacent_region_connections.swap(idx, region.adjacent_region_connection_count as usize);
        } else {
            idx += 1;
        }
    }
}

#[inline]
fn add_adjacent_connection_for_region(
    region: &mut LocalRegion,
    chunk_adjacent_region_connections: &mut [AdjacentRegionConnection],
    max_adjacent_region_connections: LocalRegionCount,
    adjacent_region_idx: LocalRegionCount,
    face_dim: Dimension,
    face_side: Side,
) {
    chunk_adjacent_region_connections
        [region.push_adjacent_region_connection_idx(max_adjacent_region_connections)] =
        AdjacentRegionConnection::new(adjacent_region_idx, face_dim, face_side);
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

    pub fn fuzz_test_voxel_object_connected_regions(generator: SDFVoxelGenerator<Global>) {
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_region_count();
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
            .split_off_any_disconnected_region_with_property_transferrer(
                &mut inertial_property_transferrer,
            )
        {
            let DisconnectedVoxelObject {
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
        assert!(object.split_off_any_disconnected_region().is_some());
    }
}
