//! Chunked representation of voxel objects.

pub mod disconnection;
pub mod inertia;
pub mod intersection;
pub mod sdf;

use crate::{
    Voxel, VoxelFlags,
    generation::ChunkedVoxelGenerator,
    utils::{DataLoop3, Dimension, Loop3, MutDataLoop3, Side},
    voxel_types::{VoxelType, VoxelTypeRegistry},
};
use bitflags::bitflags;
use bytemuck::Zeroable;
use cfg_if::cfg_if;
use disconnection::{
    NonUniformChunkSplitDetectionData, SplitDetector, UniformChunkSplitDetectionData,
};
use impact_alloc::{AVec, arena::ArenaPool};
use impact_containers::HashSet;
use impact_geometry::{AxisAlignedBox, Sphere};
use impact_math::point::Point3;
use impact_thread::{
    channel::{self, Sender},
    pool::{DynamicTask, DynamicThreadPool},
};
use num_traits::{NumCast, PrimInt};
use std::{array, iter, mem, ops::Range};
use tinyvec::TinyVec;

/// An object represented by a grid of voxels.
///
/// The grid is subdivided into cubic chunks that are [`CHUNK_SIZE`] voxels
/// across. The full grid for the object spans a whole number of chunks along
/// each axis.
///
/// Uniform voxel information is pulled up to the chunk level. An empty chunk
/// does not store any information on the voxel level, and a chunk where all
/// voxels contain the exact same information only stores that single voxel.
#[derive(Clone, Debug)]
pub struct ChunkedVoxelObject {
    voxel_extent: f32,
    inverse_voxel_extent: f32,
    chunk_counts: [usize; 3],
    chunk_idx_strides: [usize; 3],
    occupied_chunk_ranges: [Range<usize>; 3],
    occupied_voxel_ranges: [Range<usize>; 3],
    origin_offset_in_root: [usize; 3],
    chunks: Vec<VoxelChunk>,
    voxels: Vec<Voxel>,
    split_detector: SplitDetector,
    invalidated_mesh_chunk_indices: HashSet<[usize; 3]>,
}

/// A voxel chunk that is not fully obscured by adjacent voxels.
#[derive(Clone, Debug)]
pub struct ExposedVoxelChunk {
    chunk_indices: [usize; 3],
    flags: VoxelChunkFlags,
}

/// A chunk representing a cubic grid of voxels. It has three representations:
///
/// - Empty: The chunk contains no voxels.
///
/// - Uniform: The chunk is fully packed with voxels carrying the exact same
///   information. Only the single representative voxel is stored. Since voxels
///   carry adjacency information, boundary voxels in a uniform chunk must have
///   the same adjacencies as interior voxels, meaning that the chunk boundaries
///   must be fully obscured by adjacent chunks for the chunk to be considered
///   uniform.
///
/// - Non-uniform: The chunk is not fully packed and/or contains a mix of voxels
///   with different information. The voxels comprising the non-uniform chunk
///   are stored in the parent [`ChunkedVoxelObject`], and the chunk stores an
///   offset to its voxel data as well as information on the distribution of
///   voxels across the faces of the chunk and a set of flags encoding
///   additional information about the state of the chunk.
#[derive(Clone, Copy, Debug)]
pub enum VoxelChunk {
    Empty,
    Uniform(UniformVoxelChunk),
    NonUniform(NonUniformVoxelChunk),
}

/// A uniform chunk representing a cubic grid of voxel chunks. The chunk is
/// fully packed with voxels carrying the exact same information.
/// Only the single representative voxel is stored. Since voxels carry adjacency
/// information, boundary voxels in a uniform chunk must have the same
/// adjacencies as interior voxels, meaning that the chunk boundaries must be
/// fully obscured by adjacent chunks for the chunk to be considered uniform.
#[derive(Clone, Copy, Debug, Default)]
pub struct UniformVoxelChunk {
    voxel: Voxel,
    split_detection: UniformChunkSplitDetectionData,
}

/// A non-uniform chunk representing a cubic grid of voxel chunks. The chunk is
/// not fully packed and/or contains a mix of voxels with different information.
/// The voxels comprising the non-uniform chunk are stored in the parent
/// [`ChunkedVoxelObject`], and the chunk stores an offset to its voxel data as
/// well as information on the distribution of voxels across the faces of the
/// chunk and a set of flags encoding additional information about the state of
/// the chunk.
#[derive(Clone, Copy, Debug, Default)]
pub struct NonUniformVoxelChunk {
    data_offset: u32,
    face_distributions: [[FaceVoxelDistribution; 2]; 3],
    flags: VoxelChunkFlags,
    split_detection: NonUniformChunkSplitDetectionData,
}

#[derive(Clone, Debug)]
struct ChunkAnalysisResults {
    uniform_chunk_count: usize,
    non_uniform_chunk_count: usize,
    occupied_chunk_ranges: [Range<usize>; 3],
    occupied_voxel_ranges: [Range<usize>; 3],
}

/// Information about the distribution of voxels across a specific face of a
/// chunk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
enum FaceVoxelDistribution {
    /// There are no voxels on the face.
    #[default]
    Empty,
    /// The face is completely filled with voxels (but they may have different
    /// properties).
    Full,
    /// The face is partially filled with voxels.
    Mixed,
}

bitflags! {
    /// Bitflags encoding a set of potential binary states for a voxel chunk.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct VoxelChunkFlags: u8 {
        /// The face on the negative x-side of the chunk is fully obscured by
        /// adjacent voxels.
        const IS_OBSCURED_X_DN = 1 << 0;
        /// The face on the negative y-side of the chunk is fully obscured by
        /// adjacent voxels.
        const IS_OBSCURED_Y_DN = 1 << 1;
        /// The face on the negative z-side of the chunk is fully obscured by
        /// adjacent voxels.
        const IS_OBSCURED_Z_DN = 1 << 2;
        /// The face on the positive x-side of the chunk is fully obscured by
        /// adjacent voxels.
        const IS_OBSCURED_X_UP = 1 << 3;
        /// The face on the positive y-side of the chunk is fully obscured by
        /// adjacent voxels.
        const IS_OBSCURED_Y_UP = 1 << 4;
        /// The face on the positive z-side of the chunk is fully obscured by
        /// adjacent voxels.
        const IS_OBSCURED_Z_UP = 1 << 5;
        /// The chunk contains only empty voxels.
        const IS_EMPTY         = 1 << 6;
    }
}

/// Helper struct for keeping track of the number of empty voxels on each face
/// of a chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FaceEmptyCounts([[usize; 2]; 3]);

pub type LoopForChunkVoxels = Loop3<CHUNK_SIZE>;
pub type LoopOverChunkVoxelData<'a, 'b> = DataLoop3<'a, 'b, Voxel, CHUNK_SIZE>;
pub type LoopOverChunkVoxelDataMut<'a, 'b> = MutDataLoop3<'a, 'b, Voxel, CHUNK_SIZE>;

const LOG2_CHUNK_SIZE: usize = 4;

/// The minimum number of non-empty voxels that should be present in a voxel
/// object for it to be considered non-empty.
pub const NON_EMPTY_VOXEL_THRESHOLD: usize = 8;

/// The number of voxels across a cubic voxel chunk. It is always a power of
/// two.
pub const CHUNK_SIZE: usize = 1 << LOG2_CHUNK_SIZE;
const CHUNK_SIZE_SQUARED: usize = CHUNK_SIZE.pow(2);
/// The total number of voxels comprising each chunk.
const CHUNK_VOXEL_COUNT: usize = CHUNK_SIZE.pow(3);

// We assume that a linear voxel index within a chunk fits into a `u16`
const _: () = assert!(CHUNK_VOXEL_COUNT <= u16::MAX as usize);

const CHUNK_IDX_FROM_OBJECT_VOXEL_IDX_SHIFT: usize = LOG2_CHUNK_SIZE;
const VOXEL_IDX_FROM_OBJECT_VOXEL_IDX_MASK: usize = (1 << LOG2_CHUNK_SIZE) - 1;

const VOXEL_INDEX_FROM_LINEAR_IDX_MASK: usize = (1 << LOG2_CHUNK_SIZE) - 1;

#[allow(clippy::reversed_empty_ranges)]
const REVERSED_MAX_RANGE: Range<usize> = usize::MAX..usize::MIN;

impl ChunkedVoxelObject {
    /// The number of voxels across a cubic voxel chunk. It is always a power of
    /// two.
    pub const fn chunk_size() -> usize {
        CHUNK_SIZE
    }

    /// The total number of voxels comprising each chunk.
    pub const fn chunk_voxel_count() -> usize {
        CHUNK_VOXEL_COUNT
    }

    /// Generates a new `ChunkedVoxelObject` using the given
    /// [`ChunkedVoxelGenerator`] and calls
    /// [`Self::update_occupied_voxel_ranges`] and
    /// [`Self::compute_all_derived_state`] on it.
    pub fn generate(generator: &impl ChunkedVoxelGenerator) -> Self {
        let mut object = Self::generate_without_derived_state(generator);
        object.update_occupied_voxel_ranges();
        object.compute_all_derived_state();
        object
    }

    /// Generates a new `ChunkedVoxelObject` using the given
    /// [`ChunkedVoxelGenerator`] and calls
    /// [`Self::update_occupied_voxel_ranges`] and
    /// [`Self::compute_all_derived_state`] on it.
    pub fn generate_in_parallel<G>(thread_pool: &DynamicThreadPool, generator: &G) -> Self
    where
        G: ChunkedVoxelGenerator + Sync,
    {
        let mut object = Self::generate_without_derived_state_in_parallel(thread_pool, generator);
        object.update_occupied_voxel_ranges();
        object.compute_all_derived_state();
        object
    }

    /// Generates a new `ChunkedVoxelObject` using the given
    /// [`ChunkedVoxelGenerator`].
    pub fn generate_without_derived_state(generator: &impl ChunkedVoxelGenerator) -> Self {
        Self::generate_without_derived_state_using_closure(
            generator.voxel_extent(),
            generator.grid_shape(),
            |chunk_counts, chunks| {
                Self::generate_voxels_for_chunks(generator, chunk_counts, chunks)
            },
        )
    }

    /// Generates a new `ChunkedVoxelObject` using the given
    /// [`ChunkedVoxelGenerator`].
    pub fn generate_without_derived_state_in_parallel<G>(
        thread_pool: &DynamicThreadPool,
        generator: &G,
    ) -> Self
    where
        G: ChunkedVoxelGenerator + Sync,
    {
        Self::generate_without_derived_state_using_closure(
            generator.voxel_extent(),
            generator.grid_shape(),
            |chunk_counts, chunks| {
                Self::generate_voxels_for_chunks_in_parallel(
                    thread_pool,
                    generator,
                    chunk_counts,
                    chunks,
                )
            },
        )
    }

    fn generate_without_derived_state_using_closure(
        voxel_extent: f32,
        grid_shape: [usize; 3],
        generate_voxels_for_chunks: impl FnOnce([usize; 3], &mut [VoxelChunk]) -> Vec<Voxel>,
    ) -> Self {
        let chunk_counts = grid_shape.map(|size| size.div_ceil(CHUNK_SIZE));
        let total_chunk_count = chunk_counts.iter().product();

        let mut chunks = vec![VoxelChunk::Empty; total_chunk_count];

        let voxels = generate_voxels_for_chunks(chunk_counts, &mut chunks);

        let ChunkAnalysisResults {
            uniform_chunk_count,
            non_uniform_chunk_count,
            occupied_chunk_ranges,
            occupied_voxel_ranges,
        } = Self::analyze_and_initialize_chunks(chunk_counts, &mut chunks);

        let chunk_idx_strides = [chunk_counts[2] * chunk_counts[1], chunk_counts[2], 1];

        // This object has not been split off from a parent
        let origin_offset_in_root = [0; 3];

        let split_detector = SplitDetector::new(uniform_chunk_count, non_uniform_chunk_count);

        Self {
            voxel_extent,
            inverse_voxel_extent: voxel_extent.recip(),
            chunk_counts,
            chunk_idx_strides,
            occupied_chunk_ranges,
            occupied_voxel_ranges,
            origin_offset_in_root,
            chunks,
            voxels,
            split_detector,
            invalidated_mesh_chunk_indices: HashSet::default(),
        }
    }

    fn generate_voxels_for_chunks<G>(
        generator: &G,
        chunk_counts: [usize; 3],
        chunks: &mut [VoxelChunk],
    ) -> Vec<Voxel>
    where
        G: ChunkedVoxelGenerator,
    {
        assert_eq!(chunks.len(), chunk_counts.iter().product::<usize>());

        if chunks.is_empty() {
            return Vec::new();
        }

        // Assume roughly one quarter of the chunks will be non-uniform
        let estimated_voxel_count = (chunks.len() * CHUNK_VOXEL_COUNT) / 4;
        let mut voxels = Vec::with_capacity(estimated_voxel_count);

        let arena = ArenaPool::get_arena_for_capacity(generator.total_buffer_size());
        let mut generation_buffers = generator.create_buffers_in(&arena);

        for (chunk_idx, chunk) in chunks.iter_mut().enumerate() {
            let origin =
                chunk_indices_from_linear_idx(&chunk_counts, chunk_idx).map(|i| i * CHUNK_SIZE);

            let old_voxel_count = voxels.len();
            let new_voxel_count = old_voxel_count + CHUNK_VOXEL_COUNT;

            voxels.resize(new_voxel_count, Voxel::zeroed());

            let chunk_voxels = &mut voxels[old_voxel_count..];

            generator.generate_chunk(&mut generation_buffers, chunk_voxels, &origin);

            *chunk = VoxelChunk::for_voxels(chunk_voxels);

            if matches!(chunk, VoxelChunk::Empty | VoxelChunk::Uniform(_)) {
                voxels.truncate(old_voxel_count);
            }
        }

        voxels
    }

    fn generate_voxels_for_chunks_in_parallel<G>(
        thread_pool: &DynamicThreadPool,
        generator: &G,
        chunk_counts: [usize; 3],
        chunks: &mut [VoxelChunk],
    ) -> Vec<Voxel>
    where
        G: ChunkedVoxelGenerator + Sync,
    {
        assert_eq!(chunks.len(), chunk_counts.iter().product::<usize>());

        if chunks.is_empty() {
            return Vec::new();
        }

        let num_threads = thread_pool.n_workers().get();
        let num_chunks = chunks.len();
        let chunks_per_thread = num_chunks.div_ceil(num_threads);
        // Number of slices `chunks.chunks_mut(chunks_per_thread)` will produce
        let num_tasks = num_chunks / chunks_per_thread;

        let mut voxels = Vec::new();

        thread_pool
            .with_scope(|scope| {
                // Create channel for workers to send generated voxel counts to
                // main thread
                let (count_sender, count_receiver) = channel::bounded(num_tasks);

                // Also create channels for the main thread to send mutable
                // slices of the final `voxels` vector to the workers. We need a
                // separate channel per worker.
                let mut slice_senders =
                    TinyVec::<[Option<Sender<&mut [Voxel]>>; 32]>::with_capacity(num_tasks);
                let mut slice_receivers = TinyVec::<[_; 32]>::with_capacity(num_tasks);

                for _ in 0..num_tasks {
                    let (slice_sender, slice_receiver) = channel::bounded(1);
                    slice_senders.push(Some(slice_sender));
                    slice_receivers.push(Some(slice_receiver));
                }

                scope
                    .execute(
                        chunks
                            .chunks_mut(chunks_per_thread)
                            .zip(slice_receivers)
                            .enumerate()
                            .map(|(task_idx, (chunks, slice_receiver))| {
                                let count_sender = count_sender.clone();
                                DynamicTask::new(move |_| {
                                    // Assume roughly one quarter of the chunks
                                    // will be non-uniform. This could be way
                                    // off locally even if it is true on
                                    // average, but since we are re-using arena
                                    // memory this isn't that important to get
                                    // right.
                                    let estimated_voxel_count =
                                        (chunks.len() * CHUNK_VOXEL_COUNT) / 4;

                                    let arena = ArenaPool::get_arena_for_capacity(
                                        estimated_voxel_count * mem::size_of::<Voxel>()
                                            + generator.total_buffer_size(),
                                    );

                                    let mut voxels =
                                        AVec::with_capacity_in(estimated_voxel_count, &arena);

                                    let mut generation_buffers =
                                        generator.create_buffers_in(&arena);

                                    for (local_chunk_idx, chunk) in chunks.iter_mut().enumerate() {
                                        let chunk_idx =
                                            task_idx * chunks_per_thread + local_chunk_idx;

                                        let origin =
                                            chunk_indices_from_linear_idx(&chunk_counts, chunk_idx)
                                                .map(|i| i * CHUNK_SIZE);

                                        let old_voxel_count = voxels.len();
                                        let new_voxel_count = old_voxel_count + CHUNK_VOXEL_COUNT;

                                        voxels.resize(new_voxel_count, Voxel::zeroed());

                                        let chunk_voxels = &mut voxels[old_voxel_count..];

                                        generator.generate_chunk(
                                            &mut generation_buffers,
                                            chunk_voxels,
                                            &origin,
                                        );

                                        *chunk = VoxelChunk::for_voxels(chunk_voxels);

                                        // If the chunk turned out to be empty
                                        // or uniform, we must truncate away the
                                        // generated voxels
                                        if matches!(
                                            chunk,
                                            VoxelChunk::Empty | VoxelChunk::Uniform(_)
                                        ) {
                                            voxels.truncate(old_voxel_count);
                                        }
                                    }

                                    // Send the final number of voxels generated
                                    // by this task to the main thread, along
                                    // with the identifying task index
                                    count_sender.send((task_idx, voxels.len())).unwrap();

                                    // Wait for the main thread to send back the
                                    // appropriate slice (not arena-allocated)
                                    // to write the generated voxels into
                                    let slice = slice_receiver.unwrap().recv().unwrap();
                                    slice.copy_from_slice(&voxels);
                                })
                            }),
                    )
                    .unwrap();

                // Generated voxel counts by task index
                let mut counts = TinyVec::<[usize; 32]>::new();
                counts.resize(num_tasks, 0);

                let mut total_voxel_count = 0;

                for _ in 0..num_tasks {
                    let (task_idx, voxel_count_for_task) = count_receiver.recv().unwrap();
                    counts[task_idx] = voxel_count_for_task;
                    total_voxel_count += voxel_count_for_task;
                }

                // Resize the final voxel vector to the correct size, which we
                // now know exactly
                voxels.resize(total_voxel_count, Voxel::zeroed());

                // Split up the voxel slice into the appropriate segments and
                // send to the correct workers so they can write their results
                // into them
                let mut remaining_voxels = voxels.as_mut_slice();
                for (count, slice_sender) in counts.into_iter().zip(slice_senders) {
                    let (head, tail) = remaining_voxels.split_at_mut(count);
                    remaining_voxels = tail;
                    slice_sender.unwrap().send(head).unwrap();
                }
            })
            .unwrap();

        voxels
    }

    fn analyze_and_initialize_chunks(
        chunk_counts: [usize; 3],
        chunks: &mut [VoxelChunk],
    ) -> ChunkAnalysisResults {
        let mut occupied_chunks_i = REVERSED_MAX_RANGE;
        let mut occupied_chunks_j = REVERSED_MAX_RANGE;
        let mut occupied_chunks_k = REVERSED_MAX_RANGE;

        let mut uniform_chunk_count = 0;
        let mut non_uniform_chunk_count = 0;
        let mut has_non_empty_chunks = false;

        for (chunk_idx, chunk) in chunks.iter_mut().enumerate() {
            match chunk {
                VoxelChunk::Uniform(uniform_chunk) => {
                    uniform_chunk.split_detection =
                        UniformChunkSplitDetectionData::new(uniform_chunk_count);
                    uniform_chunk_count += 1;
                }
                VoxelChunk::NonUniform(non_uniform_chunk) => {
                    non_uniform_chunk.data_offset = non_uniform_chunk_count as u32;
                    non_uniform_chunk_count += 1;
                }
                VoxelChunk::Empty => {}
            }

            if !chunk.contains_only_empty_voxels() {
                let [chunk_i, chunk_j, chunk_k] =
                    chunk_indices_from_linear_idx(&chunk_counts, chunk_idx);

                occupied_chunks_i.start = occupied_chunks_i.start.min(chunk_i);
                occupied_chunks_i.end = occupied_chunks_i.end.max(chunk_i + 1);
                occupied_chunks_j.start = occupied_chunks_j.start.min(chunk_j);
                occupied_chunks_j.end = occupied_chunks_j.end.max(chunk_j + 1);
                occupied_chunks_k.start = occupied_chunks_k.start.min(chunk_k);
                occupied_chunks_k.end = occupied_chunks_k.end.max(chunk_k + 1);

                has_non_empty_chunks = true;
            }
        }

        let occupied_chunk_ranges = if has_non_empty_chunks {
            [occupied_chunks_i, occupied_chunks_j, occupied_chunks_k]
        } else {
            [0..0, 0..0, 0..0]
        };

        let occupied_voxel_ranges = occupied_chunk_ranges
            .clone()
            .map(|chunk_range| chunk_range.start * CHUNK_SIZE..chunk_range.end * CHUNK_SIZE);

        ChunkAnalysisResults {
            uniform_chunk_count,
            non_uniform_chunk_count,
            occupied_chunk_ranges,
            occupied_voxel_ranges,
        }
    }

    fn _find_voxel_types(chunks: &[VoxelChunk], voxels: &[Voxel]) -> Vec<VoxelType> {
        let mut has_voxel_type = [false; VoxelTypeRegistry::max_n_voxel_types() + 1];

        for chunk in chunks {
            if let VoxelChunk::Uniform(UniformVoxelChunk { voxel, .. }) = chunk {
                has_voxel_type[voxel.voxel_type().idx()] = true;
            }
        }
        for voxel in voxels {
            has_voxel_type[voxel.voxel_type().idx()] = true;
        }

        has_voxel_type[..VoxelTypeRegistry::max_n_voxel_types()]
            .iter()
            .enumerate()
            .filter_map(|(idx, &has_voxel_type)| {
                if has_voxel_type {
                    Some(VoxelType::from_idx_u8(idx as u8))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the extent of single voxel in the object.
    #[inline]
    pub fn voxel_extent(&self) -> f32 {
        self.voxel_extent
    }

    /// Returns the reciprocal of the voxel extent.
    #[inline]
    pub fn inverse_voxel_extent(&self) -> f32 {
        self.inverse_voxel_extent
    }

    /// Returns the extent of single voxel chunk in the object.
    #[inline]
    pub fn chunk_extent(&self) -> f32 {
        self.voxel_extent * CHUNK_SIZE as f32
    }

    /// Returns the number of chunks along each axis of the object's voxel
    /// grid.
    #[inline]
    pub fn chunk_counts(&self) -> &[usize; 3] {
        &self.chunk_counts
    }

    /// Returns the total number of chunks, incuding empty ones, contained in
    /// the object's chunk grid.
    #[inline]
    pub fn total_chunk_count(&self) -> usize {
        self.chunk_counts.iter().product()
    }

    /// Returns a guess for the rough number of exposed chunks the object
    /// contains based on its size.
    #[inline]
    pub fn exposed_chunk_count_heuristic(&self) -> usize {
        // It is probably roughly equal to the total number of boundary chunks
        2 * (self.chunk_counts[0] * self.chunk_counts[1]
            + self.chunk_counts[1] * self.chunk_counts[2]
            + self.chunk_counts[2] * self.chunk_counts[0])
    }

    /// Returns a guess for the rough number of surface voxels the object
    /// contains based on its size.
    #[inline]
    pub fn surface_voxel_count_heuristic(&self) -> usize {
        // Assuming one face is fully exposed
        CHUNK_SIZE_SQUARED * self.exposed_chunk_count_heuristic()
    }

    /// Returns the range of indices along each axis of the object's chunk
    /// grid that may contain non-empty chunks.
    #[inline]
    pub fn occupied_chunk_ranges(&self) -> &[Range<usize>; 3] {
        &self.occupied_chunk_ranges
    }

    /// Returns the range of indices along each axis of the object's voxel
    /// grid that may contain non-empty voxels.
    #[inline]
    pub fn occupied_voxel_ranges(&self) -> &[Range<usize>; 3] {
        &self.occupied_voxel_ranges
    }

    /// Whether the object only consists of empty voxels.
    #[inline]
    pub fn contains_only_empty_voxels(&self) -> bool {
        self.occupied_voxel_ranges.iter().any(Range::is_empty)
    }

    /// Returns the stride in the linear chunk index correponding to
    /// incrementing each 3D chunk index.
    #[inline]
    pub fn chunk_idx_strides(&self) -> &[usize; 3] {
        &self.chunk_idx_strides
    }

    /// Returns the offsets of the origin of this object compared to the origin
    /// of the original unsplit object this object was disconnected from, in the
    /// reference frame of the original object (the disconnected object has the
    /// same orientation as the original object after splitting, only the offset
    /// is different). This does not account for any relative motion of the
    /// objects after splitting. If this object has not been disconnected from a
    /// larger object, the offsets are zero.
    #[inline]
    pub fn origin_offset_in_root(&self) -> [f32; 3] {
        self.origin_offset_in_root
            .map(|offset| self.voxel_extent * offset as f32)
    }

    /// Determines the exact range of indices along each axis of the object's
    /// voxel grid that may contain non-empty voxels.
    pub fn determine_tight_occupied_voxel_ranges(&self) -> [Range<usize>; 3] {
        determine_occupied_voxel_ranges(self.chunk_counts, &self.chunks, &self.voxels)
    }

    /// Returns the number of voxels (potentially empty) actually stored in the
    /// object (as opposed to the count of voxels the object logically
    /// contains).
    #[inline]
    pub fn stored_voxel_count(&self) -> usize {
        self.chunks
            .iter()
            .map(|chunk| chunk.stored_voxel_count())
            .sum()
    }

    /// Returns the slice of all voxel chunks.
    #[inline]
    pub fn chunks(&self) -> &[VoxelChunk] {
        &self.chunks
    }

    /// Returns the slice of all stored (non-uniform chunk) voxels.
    #[inline]
    pub fn voxels(&self) -> &[Voxel] {
        &self.voxels
    }

    /// Calls the given closure for each occupied chunk in the object, passing in
    /// the chunk and its indices in the chunk grid.
    pub fn for_each_occupied_chunk(&self, f: &mut impl FnMut([usize; 3], &VoxelChunk)) {
        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_j in self.occupied_chunk_ranges[1].clone() {
                for chunk_k in self.occupied_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];

                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);
                    let chunk = &self.chunks[chunk_idx];

                    if !chunk.contains_only_empty_voxels() {
                        f(chunk_indices, chunk);
                    }
                }
            }
        }
    }

    /// Calls the given closure for each chunk in the object (occupied or not),
    /// passing in the chunk and its indices in the chunk grid.
    pub fn for_each_chunk(&self, f: &mut impl FnMut([usize; 3], &VoxelChunk)) {
        for chunk_i in 0..self.chunk_counts[0] {
            for chunk_j in 0..self.chunk_counts[1] {
                for chunk_k in 0..self.chunk_counts[2] {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];

                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);
                    let chunk = &self.chunks[chunk_idx];

                    f(chunk_indices, chunk);
                }
            }
        }
    }

    /// Checks whether the object consists of fewer than
    /// [`NON_EMPTY_VOXEL_THRESHOLD`] non-empty voxels. Assumes that
    /// [`Self::update_occupied_ranges`] has been called since the last time a
    /// chunk was emptied.
    pub fn is_effectively_empty(&self) -> bool {
        let occupied_chunk_count: usize =
            self.occupied_chunk_ranges.iter().map(Range::len).product();

        if occupied_chunk_count >= NON_EMPTY_VOXEL_THRESHOLD {
            // There is at least one non-empty voxel in each occupied chunk
            return false;
        }

        let max_occupied_voxel_count: usize =
            self.occupied_voxel_ranges.iter().map(Range::len).product();

        if max_occupied_voxel_count < NON_EMPTY_VOXEL_THRESHOLD {
            return true;
        }

        let mut total_non_empty_voxel_count = 0;

        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_j in self.occupied_chunk_ranges[1].clone() {
                for chunk_k in self.occupied_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);
                    let non_empty_voxel_count = match &self.chunks[chunk_idx] {
                        VoxelChunk::Empty => 0,
                        VoxelChunk::Uniform(_) => CHUNK_VOXEL_COUNT,
                        VoxelChunk::NonUniform(NonUniformVoxelChunk { data_offset, .. }) => {
                            let chunk_voxels = chunk_voxels(&self.voxels, *data_offset);
                            chunk_voxels
                                .iter()
                                .filter(|voxel| !voxel.is_empty())
                                .count()
                        }
                    };
                    total_non_empty_voxel_count += non_empty_voxel_count;

                    if total_non_empty_voxel_count >= NON_EMPTY_VOXEL_THRESHOLD {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Computes the axis-aligned bounding box enclosing all non-empty voxels in
    /// the object.
    #[inline]
    pub fn compute_aabb(&self) -> AxisAlignedBox {
        let lower_corner = Point3::new(
            self.occupied_voxel_ranges[0].start as f32 * self.voxel_extent,
            self.occupied_voxel_ranges[1].start as f32 * self.voxel_extent,
            self.occupied_voxel_ranges[2].start as f32 * self.voxel_extent,
        );

        let upper_corner = Point3::new(
            self.occupied_voxel_ranges[0].end as f32 * self.voxel_extent,
            self.occupied_voxel_ranges[1].end as f32 * self.voxel_extent,
            self.occupied_voxel_ranges[2].end as f32 * self.voxel_extent,
        );

        AxisAlignedBox::new(lower_corner, upper_corner)
    }

    /// Computes a sphere enclosing all non-empty voxels in the object.
    #[inline]
    pub fn compute_bounding_sphere(&self) -> Sphere {
        let bounding_sphere_for_outer_voxel_centers =
            Sphere::bounding_sphere_for_points(&self.compute_occupied_voxel_range_corner_centers());

        // If we add the distance from the center to the corner of a voxel, the
        // bounding sphere will encompass all voxels
        let additional_radius = 0.5 * f32::sqrt(3.0) * self.voxel_extent();

        Sphere::new(
            *bounding_sphere_for_outer_voxel_centers.center(),
            bounding_sphere_for_outer_voxel_centers.radius() + additional_radius,
        )
    }

    #[inline]
    fn compute_occupied_voxel_range_corner_centers(&self) -> [Point3; 8] {
        if self.contains_only_empty_voxels() {
            return [Point3::origin(); 8];
        }

        let rx = &self.occupied_voxel_ranges[0];
        let ry = &self.occupied_voxel_ranges[1];
        let rz = &self.occupied_voxel_ranges[2];

        [
            [rx.start, ry.start, rz.start],
            [rx.start, ry.start, rz.end - 1],
            [rx.start, ry.end - 1, rz.start],
            [rx.start, ry.end - 1, rz.end - 1],
            [rx.end - 1, ry.start, rz.start],
            [rx.end - 1, ry.start, rz.end - 1],
            [rx.end - 1, ry.end - 1, rz.start],
            [rx.end - 1, ry.end - 1, rz.end - 1],
        ]
        .map(|[i, j, k]| self.voxel_center_position_from_object_voxel_indices(i, j, k))
    }

    /// Calls the given closure for each voxel in the given non-uniform chunk,
    /// passing in the *local* 3D indices of the voxel within the chunk.
    ///
    /// # Panics
    /// May panic of the chunk's handle to its segment of the object's voxel
    /// buffer is invalid.
    pub fn for_each_voxel_in_non_uniform_chunk(
        &self,
        chunk: &NonUniformVoxelChunk,
        f: &mut impl FnMut(&[usize; 3], Voxel),
    ) {
        let voxels = self.non_uniform_chunk_voxels(chunk);
        LoopOverChunkVoxelData::new(&LoopForChunkVoxels::over_all(), voxels).execute(
            &mut |indices, voxel| {
                f(indices, *voxel);
            },
        );
    }

    /// Returns the flat slice of voxels in the given non-uniform chunk. The
    /// length of the slice is [`Self::chunk_voxel_count`].
    ///
    /// # Panics
    /// May panic of the chunk's handle to its segment of the object's voxel
    /// buffer is invalid.
    #[inline]
    pub fn non_uniform_chunk_voxels(&self, chunk: &NonUniformVoxelChunk) -> &[Voxel] {
        chunk_voxels(&self.voxels, chunk.data_offset)
    }

    /// Returns a reference to the voxel containing the given model space
    /// coordinates (fractional indices scaled by the voxel extent) in the
    /// object's voxel grid, or [`None`] if the voxel is empty or the
    /// coordinates are out of bounds.
    #[inline]
    pub fn get_voxel_at_coords(&self, x: f32, y: f32, z: f32) -> Option<&Voxel> {
        let i = (x * self.inverse_voxel_extent) as i64;
        let j = (y * self.inverse_voxel_extent) as i64;
        let k = (z * self.inverse_voxel_extent) as i64;
        self.get_voxel(i, j, k)
    }

    /// Returns a reference to the voxel at the given indices in the object's
    /// voxel grid, or [`None`] if the voxel is empty or the indices are out of
    /// bounds.
    ///
    /// Despite the organization of voxels into chunks, this lookup is
    /// relatively efficient because we can perform simple bit manipulations
    /// to determine the chunk containing the voxel.
    #[inline]
    pub fn get_voxel<I: PrimInt>(&self, i: I, j: I, k: I) -> Option<&Voxel> {
        if i < I::from(self.occupied_voxel_ranges[0].start).unwrap()
            || j < I::from(self.occupied_voxel_ranges[1].start).unwrap()
            || k < I::from(self.occupied_voxel_ranges[2].start).unwrap()
            || i >= I::from(self.occupied_voxel_ranges[0].end).unwrap()
            || j >= I::from(self.occupied_voxel_ranges[1].end).unwrap()
            || k >= I::from(self.occupied_voxel_ranges[2].end).unwrap()
        {
            return None;
        }

        let i = NumCast::from(i).unwrap();
        let j = NumCast::from(j).unwrap();
        let k = NumCast::from(k).unwrap();

        self.get_voxel_inside(i, j, k)
    }

    /// Returns a reference to the voxel at the given indices in the object's
    /// voxel grid, or [`None`] if the voxel is empty.
    ///
    /// Despite the organization of voxels into chunks, this lookup is
    /// relatively efficient because we can perform simple bit manipulations
    /// to determine the chunk containing the voxel.
    ///
    /// # Panics
    /// If the indices are outside the object's voxel grid.
    #[inline]
    pub fn get_voxel_inside(&self, i: usize, j: usize, k: usize) -> Option<&Voxel> {
        let chunk_idx = self.linear_chunk_idx_from_object_voxel_indices(i, j, k);
        let chunk = &self.chunks[chunk_idx];
        match &chunk {
            VoxelChunk::Empty => None,
            VoxelChunk::Uniform(UniformVoxelChunk { voxel, .. }) => Some(voxel),
            VoxelChunk::NonUniform(NonUniformVoxelChunk { data_offset, .. }) => {
                let voxel_idx = chunk_start_voxel_idx(*data_offset)
                    + linear_voxel_idx_within_chunk_from_object_voxel_indices(i, j, k);
                let voxel = &self.voxels[voxel_idx];
                if voxel.is_empty() { None } else { Some(voxel) }
            }
        }
    }

    /// Returns the [`VoxelChunk`] at the given indices in the object's chunk
    /// grid. If the indices are out of bounds, an empty chunk is returned.
    #[inline]
    pub fn get_chunk<I: PrimInt>(&self, chunk_i: I, chunk_j: I, chunk_k: I) -> VoxelChunk {
        if chunk_i < I::zero()
            || chunk_j < I::zero()
            || chunk_k < I::zero()
            || chunk_i >= I::from(self.chunk_counts[0]).unwrap()
            || chunk_j >= I::from(self.chunk_counts[1]).unwrap()
            || chunk_k >= I::from(self.chunk_counts[2]).unwrap()
        {
            return VoxelChunk::Empty;
        }

        let chunk_i = NumCast::from(chunk_i).unwrap();
        let chunk_j = NumCast::from(chunk_j).unwrap();
        let chunk_k = NumCast::from(chunk_k).unwrap();

        let chunk_idx = self.linear_chunk_idx(&[chunk_i, chunk_j, chunk_k]);
        self.chunks[chunk_idx]
    }

    /// Computes all derived state based on the raw voxel information in the
    /// object. This involves:
    ///
    /// - Determining the adjacency [`VoxelFlags`] for each voxel in the object
    ///   according to which of their six neighbor voxels are present.
    /// - Recording which faces of the chunks are fully obscured by adjacent
    ///   voxels.
    /// - Building the data structure for identifying whether and where the
    ///   object is split into disconnected voxel regions.
    pub fn compute_all_derived_state(&mut self) {
        self.update_internal_adjacencies_for_all_chunks();
        self.update_local_connected_regions_for_all_chunks();
        self.compute_all_chunk_external_derived_state();
    }

    fn compute_all_chunk_external_derived_state(&mut self) {
        self.update_all_chunk_boundary_adjacencies();
        self.resolve_connected_regions_between_all_chunks();
    }

    /// Updates the recorded occupied chunk and voxel ranges by checking which
    /// chunks and voxels are occupied.
    pub fn update_occupied_ranges(&mut self) {
        self.update_occupied_chunk_ranges();
        self.update_occupied_voxel_ranges();
    }

    /// Updates the recorded occupied chunk ranges by checking which chunks are
    /// occupied.
    fn update_occupied_chunk_ranges(&mut self) {
        let mut min_chunk_indices = [usize::MAX; 3];
        let mut max_chunk_indices = [0; 3];
        let mut has_non_empty_chunks = false;

        for chunk_i in 0..self.chunk_counts[0] {
            for chunk_j in 0..self.chunk_counts[1] {
                for chunk_k in 0..self.chunk_counts[2] {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);
                    if !self.chunks[chunk_idx].contains_only_empty_voxels() {
                        min_chunk_indices =
                            componentwise_min_indices(&min_chunk_indices, &chunk_indices);
                        max_chunk_indices =
                            componentwise_max_indices(&max_chunk_indices, &chunk_indices);
                        has_non_empty_chunks = true;
                    }
                }
            }
        }

        self.occupied_chunk_ranges = if has_non_empty_chunks {
            array::from_fn(|dim| min_chunk_indices[dim]..max_chunk_indices[dim] + 1)
        } else {
            [0..0, 0..0, 0..0]
        };
    }

    /// Updates the recorded occupied voxel ranges by checking which voxels in
    /// the outer occupied chunks are occupied. Make sure the occupied chunk
    /// ranges are up to date before calling this method.
    pub fn update_occupied_voxel_ranges(&mut self) {
        if self.occupied_chunk_ranges.iter().any(Range::is_empty) {
            self.occupied_voxel_ranges = [0..0, 0..0, 0..0];
            return;
        }

        self.occupied_voxel_ranges = Dimension::all().map(|dim| {
            let first = self.find_voxel_bound_for_dimension(dim, Side::Lower);
            let last = self.find_voxel_bound_for_dimension(dim, Side::Upper);
            first..last + 1
        });
    }

    /// Finds the occupied voxel bound along the specified dimension and side.
    fn find_voxel_bound_for_dimension(&self, search_dim: Dimension, side: Side) -> usize {
        let search_axis = search_dim.idx();
        let other_axes = match search_axis {
            0 => (1, 2),
            1 => (0, 2),
            2 => (0, 1),
            _ => unreachable!(),
        };

        assert!(!self.occupied_chunk_ranges[search_axis].is_empty());

        let (chunk_l, mut bound) = match side {
            Side::Lower => (self.occupied_chunk_ranges[search_axis].start, usize::MAX),
            Side::Upper => (self.occupied_chunk_ranges[search_axis].end - 1, 0),
        };

        let chunk_start_voxel = chunk_l * CHUNK_SIZE;

        for chunk_m in self.occupied_chunk_ranges[other_axes.0].clone() {
            for chunk_n in self.occupied_chunk_ranges[other_axes.1].clone() {
                let mut chunk_indices = [0; 3];
                chunk_indices[search_axis] = chunk_l;
                chunk_indices[other_axes.0] = chunk_m;
                chunk_indices[other_axes.1] = chunk_n;

                let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                match &self.chunks[chunk_idx] {
                    VoxelChunk::Empty => {}
                    VoxelChunk::Uniform(_) => {
                        return match side {
                            Side::Lower => chunk_start_voxel,
                            Side::Upper => chunk_start_voxel + CHUNK_SIZE - 1,
                        };
                    }
                    VoxelChunk::NonUniform(NonUniformVoxelChunk { data_offset, .. }) => {
                        let chunk_voxels = chunk_voxels(&self.voxels, *data_offset);

                        if let Some(bound_in_chunk) =
                            Self::find_voxel_bound_in_chunk(chunk_voxels, search_dim, side)
                        {
                            bound = match side {
                                Side::Lower => bound.min(chunk_start_voxel + bound_in_chunk),
                                Side::Upper => bound.max(chunk_start_voxel + bound_in_chunk),
                            };
                        }
                    }
                }
            }
        }

        bound
    }

    /// Finds the voxel bound within a chunk along the search dimension.
    fn find_voxel_bound_in_chunk(
        chunk_voxels: &[Voxel],
        search_dim: Dimension,
        side: Side,
    ) -> Option<usize> {
        let mut result = None;

        Loop3::<CHUNK_SIZE>::over_all_from_side(search_dim, side).execute_short_circuiting(
            &mut |i, j, k| {
                let idx = linear_voxel_idx_within_chunk(&[i, j, k]);
                if !chunk_voxels[idx].is_empty() {
                    result = Some(match search_dim {
                        Dimension::X => i,
                        Dimension::Y => j,
                        Dimension::Z => k,
                    });
                    false // Break early
                } else {
                    true // Continue
                }
            },
        );

        result
    }

    /// Returns an iterator over the indices in the object's chunk grid of the
    /// chunks whose (hypothetical) meshes have been invalidated by changes in
    /// the voxel object since the object was created or
    /// [`Self::mark_chunk_meshes_synchronized`] was last called.
    #[inline]
    pub fn invalidated_mesh_chunk_indices(&self) -> impl ExactSizeIterator<Item = &[usize; 3]> {
        self.invalidated_mesh_chunk_indices.iter()
    }

    /// Signals that the mesh data of all the object's chunks is up to date with
    /// the object's voxels.
    #[inline]
    pub fn mark_chunk_meshes_synchronized(&mut self) {
        self.invalidated_mesh_chunk_indices.clear();
    }

    /// Validates the occupied voxel ranges computed by the efficient
    /// [`Self::update_occupied_voxel_ranges`] method by performing a simple
    /// brute-force iteration over all voxels and checking their bounds.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_occupied_voxel_ranges(&self) {
        let expected_ranges = if self.occupied_chunk_ranges.iter().any(Range::is_empty) {
            [0..0, 0..0, 0..0]
        } else {
            let mut min_bounds = [usize::MAX; 3];
            let mut max_bounds = [0; 3];
            let mut found_any_voxel = false;

            // Scan all voxels in occupied chunk ranges
            for chunk_x in self.occupied_chunk_ranges[0].clone() {
                for chunk_y in self.occupied_chunk_ranges[1].clone() {
                    for chunk_z in self.occupied_chunk_ranges[2].clone() {
                        let chunk_indices = [chunk_x, chunk_y, chunk_z];
                        let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                        match &self.chunks[chunk_idx] {
                            VoxelChunk::Empty => {}
                            VoxelChunk::Uniform(_) => {
                                // Entire chunk is occupied
                                let chunk_start = [
                                    chunk_x * CHUNK_SIZE,
                                    chunk_y * CHUNK_SIZE,
                                    chunk_z * CHUNK_SIZE,
                                ];
                                let chunk_end = [
                                    chunk_start[0] + CHUNK_SIZE - 1,
                                    chunk_start[1] + CHUNK_SIZE - 1,
                                    chunk_start[2] + CHUNK_SIZE - 1,
                                ];

                                for dim in 0..3 {
                                    min_bounds[dim] = min_bounds[dim].min(chunk_start[dim]);
                                    max_bounds[dim] = max_bounds[dim].max(chunk_end[dim]);
                                }
                                found_any_voxel = true;
                            }
                            VoxelChunk::NonUniform(non_uniform_chunk) => {
                                let chunk_voxels =
                                    chunk_voxels(&self.voxels, non_uniform_chunk.data_offset);

                                // Check each voxel in the chunk
                                for i in 0..CHUNK_SIZE {
                                    for j in 0..CHUNK_SIZE {
                                        for k in 0..CHUNK_SIZE {
                                            let voxel_idx =
                                                linear_voxel_idx_within_chunk(&[i, j, k]);
                                            if !chunk_voxels[voxel_idx].is_empty() {
                                                let global_coords = [
                                                    chunk_x * CHUNK_SIZE + i,
                                                    chunk_y * CHUNK_SIZE + j,
                                                    chunk_z * CHUNK_SIZE + k,
                                                ];

                                                for dim in 0..3 {
                                                    min_bounds[dim] =
                                                        min_bounds[dim].min(global_coords[dim]);
                                                    max_bounds[dim] =
                                                        max_bounds[dim].max(global_coords[dim]);
                                                }
                                                found_any_voxel = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if found_any_voxel {
                [
                    min_bounds[0]..max_bounds[0] + 1,
                    min_bounds[1]..max_bounds[1] + 1,
                    min_bounds[2]..max_bounds[2] + 1,
                ]
            } else {
                [0..0, 0..0, 0..0]
            }
        };

        assert_eq!(
            self.occupied_voxel_ranges, expected_ranges,
            "Occupied voxel ranges are not correctly shrunk. Expected: {:?}, Found: {:?}",
            expected_ranges, self.occupied_voxel_ranges
        );
    }

    /// Validates the adjacency [`VoxelFlags`] computed by the efficient
    /// [`Self::update_internal_adjacencies_for_all_chunks`] method by
    /// performing a simple brute-force iteration over all voxels and checking
    /// their neighbors.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_adjacencies(&self) {
        let mut invalid_missing_flags = Vec::new();
        let mut invalid_present_flags = Vec::new();

        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                for k in self.occupied_voxel_ranges[2].clone() {
                    let mut assert_has_flag = |voxel: &Voxel, flag| {
                        if !voxel.is_empty() && !voxel.flags().contains(flag) {
                            invalid_missing_flags.push(([i, j, k], flag));
                        }
                    };
                    let mut assert_missing_flag = |voxel: &Voxel, flag| {
                        if voxel.flags().contains(flag) {
                            invalid_present_flags.push(([i, j, k], flag));
                        }
                    };

                    let voxel = self
                        .get_voxel(i, j, k)
                        .copied()
                        .unwrap_or(Voxel::maximally_outside());

                    let adjacent_voxel_x_up = self
                        .get_voxel(i + 1, j, k)
                        .copied()
                        .unwrap_or(Voxel::maximally_outside());
                    let adjacent_voxel_y_up = self
                        .get_voxel(i, j + 1, k)
                        .copied()
                        .unwrap_or(Voxel::maximally_outside());
                    let adjacent_voxel_z_up = self
                        .get_voxel(i, j, k + 1)
                        .copied()
                        .unwrap_or(Voxel::maximally_outside());

                    if voxel.is_empty() {
                        assert_missing_flag(&adjacent_voxel_x_up, VoxelFlags::HAS_ADJACENT_X_DN);
                        assert_missing_flag(&adjacent_voxel_y_up, VoxelFlags::HAS_ADJACENT_Y_DN);
                        assert_missing_flag(&adjacent_voxel_z_up, VoxelFlags::HAS_ADJACENT_Z_DN);
                    } else {
                        assert_has_flag(&adjacent_voxel_x_up, VoxelFlags::HAS_ADJACENT_X_DN);
                        assert_has_flag(&adjacent_voxel_y_up, VoxelFlags::HAS_ADJACENT_Y_DN);
                        assert_has_flag(&adjacent_voxel_z_up, VoxelFlags::HAS_ADJACENT_Z_DN);
                    }

                    if adjacent_voxel_x_up.is_empty() {
                        assert_missing_flag(&voxel, VoxelFlags::HAS_ADJACENT_X_UP);
                    } else {
                        assert_has_flag(&voxel, VoxelFlags::HAS_ADJACENT_X_UP);
                    }
                    if adjacent_voxel_y_up.is_empty() {
                        assert_missing_flag(&voxel, VoxelFlags::HAS_ADJACENT_Y_UP);
                    } else {
                        assert_has_flag(&voxel, VoxelFlags::HAS_ADJACENT_Y_UP);
                    }
                    if adjacent_voxel_z_up.is_empty() {
                        assert_missing_flag(&voxel, VoxelFlags::HAS_ADJACENT_Z_UP);
                    } else {
                        assert_has_flag(&voxel, VoxelFlags::HAS_ADJACENT_Z_UP);
                    }
                }
            }
        }

        for j in self.occupied_voxel_ranges[1].clone() {
            for k in self.occupied_voxel_ranges[2].clone() {
                if let Some(voxel) = self.get_voxel(0, j, k)
                    && voxel.flags().contains(VoxelFlags::HAS_ADJACENT_X_DN)
                {
                    invalid_present_flags.push(([0, j, k], VoxelFlags::HAS_ADJACENT_X_DN));
                }
            }
        }
        for i in self.occupied_voxel_ranges[0].clone() {
            for k in self.occupied_voxel_ranges[2].clone() {
                if let Some(voxel) = self.get_voxel(i, 0, k)
                    && voxel.flags().contains(VoxelFlags::HAS_ADJACENT_Y_DN)
                {
                    invalid_present_flags.push(([i, 0, k], VoxelFlags::HAS_ADJACENT_Y_DN));
                }
            }
        }
        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                if let Some(voxel) = self.get_voxel(i, j, 0)
                    && voxel.flags().contains(VoxelFlags::HAS_ADJACENT_Z_DN)
                {
                    invalid_present_flags.push(([i, j, 0], VoxelFlags::HAS_ADJACENT_Z_DN));
                }
            }
        }

        if !invalid_missing_flags.is_empty() || !invalid_present_flags.is_empty() {
            panic!(
                "Invalid adjacencies:\nMissing flags = {:?}\nPresent flags that should not be = {:?}",
                &invalid_missing_flags[..usize::min(20, invalid_missing_flags.len())],
                &invalid_present_flags[..usize::min(20, invalid_present_flags.len())]
            );
        }
    }

    /// Validates the obscuredness [`VoxelChunkFlags`] computed by the efficient
    /// [`Self::update_all_chunk_boundary_adjacencies`] method for chunks by
    /// performing a simple brute-force iteration over all chunks and checking
    /// their neighbors.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_chunk_obscuredness(&self) {
        let mut invalid_missing_flags = Vec::new();
        let mut invalid_present_flags = Vec::new();
        let mut invalid_uniform = Vec::new();

        for chunk_i in 0..self.chunk_counts[0] {
            for chunk_j in 0..self.chunk_counts[1] {
                for chunk_k in 0..self.chunk_counts[2] {
                    let mut assert_has_flag = |chunk: &VoxelChunk, flag| match chunk {
                        VoxelChunk::Empty | VoxelChunk::Uniform(_) => {}
                        VoxelChunk::NonUniform(NonUniformVoxelChunk { flags, .. }) => {
                            if !flags.contains(flag) {
                                invalid_missing_flags.push(([chunk_i, chunk_j, chunk_k], flag));
                            }
                        }
                    };
                    let mut assert_missing_flag = |chunk: &VoxelChunk, flag| match chunk {
                        VoxelChunk::Empty => {}
                        VoxelChunk::Uniform(_) => {
                            // Uniform chunks implicitly have all obscuredness flags set
                            invalid_uniform.push([chunk_i, chunk_j, chunk_k]);
                        }
                        VoxelChunk::NonUniform(NonUniformVoxelChunk { flags, .. }) => {
                            if flags.contains(flag) {
                                invalid_present_flags.push(([chunk_i, chunk_j, chunk_k], flag));
                            }
                        }
                    };

                    let chunk = self.get_chunk(chunk_i, chunk_j, chunk_k);

                    let adjacent_chunk_x_up = self.get_chunk(chunk_i + 1, chunk_j, chunk_k);
                    let adjacent_chunk_y_up = self.get_chunk(chunk_i, chunk_j + 1, chunk_k);
                    let adjacent_chunk_z_up = self.get_chunk(chunk_i, chunk_j, chunk_k + 1);

                    if chunk.upper_face_voxel_distribution(Dimension::X)
                        == FaceVoxelDistribution::Full
                    {
                        assert_has_flag(&adjacent_chunk_x_up, VoxelChunkFlags::IS_OBSCURED_X_DN);
                    } else {
                        assert_missing_flag(
                            &adjacent_chunk_x_up,
                            VoxelChunkFlags::IS_OBSCURED_X_DN,
                        );
                    }
                    if chunk.upper_face_voxel_distribution(Dimension::Y)
                        == FaceVoxelDistribution::Full
                    {
                        assert_has_flag(&adjacent_chunk_y_up, VoxelChunkFlags::IS_OBSCURED_Y_DN);
                    } else {
                        assert_missing_flag(
                            &adjacent_chunk_y_up,
                            VoxelChunkFlags::IS_OBSCURED_Y_DN,
                        );
                    }
                    if chunk.upper_face_voxel_distribution(Dimension::Z)
                        == FaceVoxelDistribution::Full
                    {
                        assert_has_flag(&adjacent_chunk_z_up, VoxelChunkFlags::IS_OBSCURED_Z_DN);
                    } else {
                        assert_missing_flag(
                            &adjacent_chunk_z_up,
                            VoxelChunkFlags::IS_OBSCURED_Z_DN,
                        );
                    }

                    if adjacent_chunk_x_up.lower_face_voxel_distribution(Dimension::X)
                        == FaceVoxelDistribution::Full
                    {
                        assert_has_flag(&chunk, VoxelChunkFlags::IS_OBSCURED_X_UP);
                    } else {
                        assert_missing_flag(&chunk, VoxelChunkFlags::IS_OBSCURED_X_UP);
                    }
                    if adjacent_chunk_y_up.lower_face_voxel_distribution(Dimension::Y)
                        == FaceVoxelDistribution::Full
                    {
                        assert_has_flag(&chunk, VoxelChunkFlags::IS_OBSCURED_Y_UP);
                    } else {
                        assert_missing_flag(&chunk, VoxelChunkFlags::IS_OBSCURED_Y_UP);
                    }
                    if adjacent_chunk_z_up.lower_face_voxel_distribution(Dimension::Z)
                        == FaceVoxelDistribution::Full
                    {
                        assert_has_flag(&chunk, VoxelChunkFlags::IS_OBSCURED_Z_UP);
                    } else {
                        assert_missing_flag(&chunk, VoxelChunkFlags::IS_OBSCURED_Z_UP);
                    }
                }
            }
        }

        for chunk_j in 0..self.chunk_counts[1] {
            for chunk_k in 0..self.chunk_counts[2] {
                match self.get_chunk(0, chunk_j, chunk_k) {
                    VoxelChunk::Empty => {}
                    VoxelChunk::Uniform(_) => {
                        invalid_uniform.push([0, chunk_j, chunk_k]);
                    }
                    VoxelChunk::NonUniform(NonUniformVoxelChunk { flags, .. }) => {
                        if flags.contains(VoxelChunkFlags::IS_OBSCURED_X_DN) {
                            invalid_present_flags
                                .push(([0, chunk_j, chunk_k], VoxelChunkFlags::IS_OBSCURED_X_DN));
                        }
                    }
                }
            }
        }
        for chunk_i in 0..self.chunk_counts[0] {
            for chunk_k in 0..self.chunk_counts[2] {
                match self.get_chunk(chunk_i, 0, chunk_k) {
                    VoxelChunk::Empty => {}
                    VoxelChunk::Uniform(_) => {
                        invalid_uniform.push([chunk_i, 0, chunk_k]);
                    }
                    VoxelChunk::NonUniform(NonUniformVoxelChunk { flags, .. }) => {
                        if flags.contains(VoxelChunkFlags::IS_OBSCURED_Y_DN) {
                            invalid_present_flags
                                .push(([chunk_i, 0, chunk_k], VoxelChunkFlags::IS_OBSCURED_Y_DN));
                        }
                    }
                }
            }
        }
        for chunk_i in 0..self.chunk_counts[0] {
            for chunk_j in 0..self.chunk_counts[1] {
                match self.get_chunk(chunk_i, chunk_j, 0) {
                    VoxelChunk::Empty => {}
                    VoxelChunk::Uniform(_) => {
                        invalid_uniform.push([chunk_i, chunk_j, 0]);
                    }
                    VoxelChunk::NonUniform(NonUniformVoxelChunk { flags, .. }) => {
                        if flags.contains(VoxelChunkFlags::IS_OBSCURED_Z_DN) {
                            invalid_present_flags
                                .push(([chunk_i, chunk_j, 0], VoxelChunkFlags::IS_OBSCURED_Z_DN));
                        }
                    }
                }
            }
        }

        if !invalid_missing_flags.is_empty() || !invalid_present_flags.is_empty() {
            panic!(
                "Invalid chunk obscuredness:\nMissing flags = {:?}\nPresent flags that should not be = {:?}",
                &invalid_missing_flags[..usize::min(20, invalid_missing_flags.len())],
                &invalid_present_flags[..usize::min(20, invalid_present_flags.len())]
            );
        }
        if !invalid_uniform.is_empty() {
            panic!(
                "Invalid uniform chunks:\nUniform chunks not completely obscured = {:?}",
                &invalid_uniform[..usize::min(20, invalid_uniform.len())]
            );
        }
    }

    pub fn update_internal_adjacencies_for_all_chunks(&mut self) {
        for chunk in &self.chunks {
            chunk.update_internal_adjacencies(self.voxels.as_mut_slice());
        }
    }

    pub fn update_all_chunk_boundary_adjacencies(&mut self) {
        // We can't constrain the chunk ranges to `self.occupied_chunk_ranges`
        // here, since there may be non-uniform chunks with only empty voxels
        // that also need their boundary adjacencies updated

        self.update_upper_boundary_adjacencies_for_chunks_in_ranges(
            self.chunk_counts.map(|count| 0..count),
        );

        // Handle lower faces of the full object, since these are not included
        // in the loop above
        for chunk_j in 0..self.chunk_counts[1] {
            for chunk_k in 0..self.chunk_counts[2] {
                let chunk_idx = self.linear_chunk_idx(&[0, chunk_j, chunk_k]);
                VoxelChunk::update_mutual_face_adjacencies(
                    &mut self.chunks,
                    &mut self.voxels,
                    &mut self.split_detector,
                    None,
                    Some(chunk_idx),
                    Dimension::X,
                );
            }
        }
        for chunk_i in 0..self.chunk_counts[0] {
            for chunk_k in 0..self.chunk_counts[2] {
                let chunk_idx = self.linear_chunk_idx(&[chunk_i, 0, chunk_k]);
                VoxelChunk::update_mutual_face_adjacencies(
                    &mut self.chunks,
                    &mut self.voxels,
                    &mut self.split_detector,
                    None,
                    Some(chunk_idx),
                    Dimension::Y,
                );
            }
        }
        for chunk_i in 0..self.chunk_counts[0] {
            for chunk_j in 0..self.chunk_counts[1] {
                let chunk_idx = self.linear_chunk_idx(&[chunk_i, chunk_j, 0]);
                VoxelChunk::update_mutual_face_adjacencies(
                    &mut self.chunks,
                    &mut self.voxels,
                    &mut self.split_detector,
                    None,
                    Some(chunk_idx),
                    Dimension::Z,
                );
            }
        }
    }

    fn update_upper_boundary_adjacencies_for_chunks_in_ranges(
        &mut self,
        chunk_ranges: [Range<usize>; 3],
    ) {
        for chunk_i in chunk_ranges[0].clone() {
            for chunk_j in chunk_ranges[1].clone() {
                for chunk_k in chunk_ranges[2].clone() {
                    let chunk_idx = self.linear_chunk_idx(&[chunk_i, chunk_j, chunk_k]);

                    for (adjacent_chunk_indices, dim) in [
                        ([chunk_i + 1, chunk_j, chunk_k], Dimension::X),
                        ([chunk_i, chunk_j + 1, chunk_k], Dimension::Y),
                        ([chunk_i, chunk_j, chunk_k + 1], Dimension::Z),
                    ] {
                        let upper_chunk_idx = if adjacent_chunk_indices[dim.idx()]
                            < self.occupied_chunk_ranges[dim.idx()].end
                        {
                            let adjacent_chunk_idx = self.linear_chunk_idx(&adjacent_chunk_indices);

                            Some(adjacent_chunk_idx)
                        } else {
                            None
                        };

                        VoxelChunk::update_mutual_face_adjacencies(
                            &mut self.chunks,
                            &mut self.voxels,
                            &mut self.split_detector,
                            Some(chunk_idx),
                            upper_chunk_idx,
                            dim,
                        );
                    }
                }
            }
        }
    }

    /// Computes the index in `self.chunks` of the chunk containing
    /// the voxel at the given indices into the object's voxel grid.
    #[inline]
    fn linear_chunk_idx_from_object_voxel_indices(&self, i: usize, j: usize, k: usize) -> usize {
        let chunk_indices = chunk_indices_from_object_voxel_indices(i, j, k);
        self.linear_chunk_idx(&chunk_indices)
    }

    /// Computes the index in `self.chunks` of the chunk with the given 3D index
    /// in the object's chunk grid.
    #[inline]
    fn linear_chunk_idx(&self, chunk_indices: &[usize; 3]) -> usize {
        chunk_indices[0] * self.chunk_idx_strides[0]
            + chunk_indices[1] * self.chunk_idx_strides[1]
            + chunk_indices[2]
    }
}

impl VoxelChunk {
    fn for_voxels(chunk_voxels: &[Voxel]) -> Self {
        assert_eq!(chunk_voxels.len(), CHUNK_VOXEL_COUNT);

        let mut first_voxel = chunk_voxels[0];
        let mut is_uniform = true;
        let mut has_non_empty_voxels = false;

        let mut face_empty_counts = FaceEmptyCounts::zero();

        LoopOverChunkVoxelData::new(&LoopForChunkVoxels::over_all(), chunk_voxels).execute(
            &mut |&[i_in_chunk, j_in_chunk, k_in_chunk], voxel| {
                if is_uniform
                    && (!voxel.matches_type_and_flags(first_voxel)
                        || !voxel.signed_distance().is_maximally_inside_or_outside())
                {
                    is_uniform = false;
                }

                if voxel.is_empty() {
                    if i_in_chunk == 0 {
                        face_empty_counts.increment_x_dn();
                    } else if i_in_chunk == CHUNK_SIZE - 1 {
                        face_empty_counts.increment_x_up();
                    }
                    if j_in_chunk == 0 {
                        face_empty_counts.increment_y_dn();
                    } else if j_in_chunk == CHUNK_SIZE - 1 {
                        face_empty_counts.increment_y_up();
                    }
                    if k_in_chunk == 0 {
                        face_empty_counts.increment_z_dn();
                    } else if k_in_chunk == CHUNK_SIZE - 1 {
                        face_empty_counts.increment_z_up();
                    }
                } else {
                    has_non_empty_voxels = true;
                }
            },
        );

        if is_uniform {
            if has_non_empty_voxels {
                // If the chunk has truly uniform information, even the boundary voxels must be
                // fully surrounded by neighbors. We don't know if this is the case yet, but we
                // assume it to be true and fix it by making the chunk non-uniform later if it
                // turns out not to be the case
                first_voxel.add_flags(VoxelFlags::full_adjacency());

                let chunk = UniformVoxelChunk {
                    voxel: first_voxel,
                    // Remaining fields will be determined later
                    ..Default::default()
                };

                Self::Uniform(chunk)
            } else {
                Self::Empty
            }
        } else {
            let face_distributions = face_empty_counts.to_chunk_face_distributions();

            let mut flags = VoxelChunkFlags::empty();
            if !has_non_empty_voxels {
                flags |= VoxelChunkFlags::IS_EMPTY;
            }

            Self::NonUniform(NonUniformVoxelChunk {
                face_distributions,
                flags,
                // Remaining fields will be determined later
                ..Default::default()
            })
        }
    }

    #[inline]
    const fn contains_only_empty_voxels(&self) -> bool {
        matches!(self, Self::Empty)
            || matches!(self, Self::NonUniform(chunk) if chunk.contains_only_empty_voxels())
    }

    #[inline]
    const fn data_offset_and_split_detection_if_non_uniform(
        &self,
    ) -> Option<(u32, NonUniformChunkSplitDetectionData)> {
        if let Self::NonUniform(NonUniformVoxelChunk {
            data_offset,
            split_detection,
            ..
        }) = self
        {
            Some((*data_offset, *split_detection))
        } else {
            None
        }
    }

    #[inline]
    const fn stored_voxel_count(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Uniform(_) => 1,
            Self::NonUniform(_) => CHUNK_VOXEL_COUNT,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn upper_face_voxel_distribution(&self, dim: Dimension) -> FaceVoxelDistribution {
        match self {
            Self::Empty => FaceVoxelDistribution::Empty,
            Self::Uniform(_) => FaceVoxelDistribution::Full,
            Self::NonUniform(NonUniformVoxelChunk {
                face_distributions, ..
            }) => face_distributions[dim.idx()][1],
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn lower_face_voxel_distribution(&self, dim: Dimension) -> FaceVoxelDistribution {
        match self {
            Self::Empty => FaceVoxelDistribution::Empty,
            Self::Uniform(_) => FaceVoxelDistribution::Full,
            Self::NonUniform(NonUniformVoxelChunk {
                face_distributions, ..
            }) => face_distributions[dim.idx()][0],
        }
    }

    fn update_internal_adjacencies(&self, voxels: &mut [Voxel]) {
        // We only need to update the internal adjacency if the chunk is
        // non-uniform
        if let Self::NonUniform(chunk) = self {
            chunk.update_internal_adjacencies(voxels);
        }
    }

    #[inline]
    fn mark_lower_face_as_obscured(&mut self, dim: Dimension) {
        let flags = match self {
            Self::Empty | Self::Uniform(_) => {
                return;
            }
            Self::NonUniform(NonUniformVoxelChunk { flags, .. }) => flags,
        };
        flags.mark_lower_face_as_obscured(dim);
    }

    #[inline]
    fn mark_upper_face_as_obscured(&mut self, dim: Dimension) {
        let flags = match self {
            Self::Empty | Self::Uniform(_) => {
                return;
            }
            Self::NonUniform(NonUniformVoxelChunk { flags, .. }) => flags,
        };
        flags.mark_upper_face_as_obscured(dim);
    }

    #[inline]
    fn mark_lower_face_as_unobscured(&mut self, dim: Dimension) {
        let flags = match self {
            Self::Empty => {
                return;
            }
            Self::Uniform(_) => {
                panic!("Tried to mark lower face of uniform chunk as unobscured");
            }
            Self::NonUniform(NonUniformVoxelChunk { flags, .. }) => flags,
        };
        flags.mark_lower_face_as_unobscured(dim);
    }

    #[inline]
    fn mark_upper_face_as_unobscured(&mut self, dim: Dimension) {
        let flags = match self {
            Self::Empty => {
                return;
            }
            Self::Uniform(_) => {
                panic!("Tried to mark upper face of uniform chunk as unobscured");
            }
            Self::NonUniform(NonUniformVoxelChunk { flags, .. }) => flags,
        };
        flags.mark_upper_face_as_unobscured(dim);
    }

    fn update_mutual_face_adjacencies(
        chunks: &mut [VoxelChunk],
        voxels: &mut Vec<Voxel>,
        split_detector: &mut SplitDetector,
        lower_chunk_idx: Option<usize>,
        upper_chunk_idx: Option<usize>,
        dim: Dimension,
    ) {
        let lower_chunk =
            lower_chunk_idx.map_or_else(|| VoxelChunk::Empty, |chunk_idx| chunks[chunk_idx]);
        let upper_chunk =
            upper_chunk_idx.map_or_else(|| VoxelChunk::Empty, |chunk_idx| chunks[chunk_idx]);

        match (lower_chunk, upper_chunk) {
            // If both chunks are empty, there is nothing to do
            (Self::Empty, Self::Empty) => {}
            // If both chunks are uniform, we don't have to update their obscuredness (uniform
            // chunks are always marked as fully obscured upon creation), but the split detector
            // still needs to perform an update
            (
                Self::Uniform(UniformVoxelChunk {
                    split_detection: lower_chunk_split_detection,
                    ..
                }),
                Self::Uniform(UniformVoxelChunk {
                    split_detection: upper_chunk_split_detection,
                    ..
                }),
            ) => {
                split_detector.update_mutual_connections_for_uniform_chunks(
                    lower_chunk_split_detection,
                    upper_chunk_split_detection,
                    dim,
                );
            }
            // If one is uniform and the other is empty, we need to convert the
            // uniform chunk to non-uniform and clear its adjacencies to the
            // empty chunk, as well as mark the adjoining face of the uniform
            // chunk as unobscured
            (Self::Uniform(_), Self::Empty) => {
                let lower_chunk = &mut chunks[lower_chunk_idx.unwrap()];
                lower_chunk.convert_to_non_uniform_if_uniform(voxels, split_detector);

                let (lower_chunk_data_offset, lower_chunk_split_detection) = lower_chunk
                    .data_offset_and_split_detection_if_non_uniform()
                    .unwrap();

                Self::remove_all_outward_adjacencies_for_face(
                    voxels,
                    lower_chunk_data_offset,
                    dim,
                    Side::Upper,
                );

                split_detector.remove_connections_for_non_uniform_chunk(
                    lower_chunk_data_offset,
                    lower_chunk_split_detection,
                    dim,
                    Side::Upper,
                );

                lower_chunk.mark_upper_face_as_unobscured(dim);
            }
            (Self::Empty, Self::Uniform(_)) => {
                let upper_chunk = &mut chunks[upper_chunk_idx.unwrap()];
                upper_chunk.convert_to_non_uniform_if_uniform(voxels, split_detector);

                let (upper_chunk_data_offset, upper_chunk_split_detection) = upper_chunk
                    .data_offset_and_split_detection_if_non_uniform()
                    .unwrap();

                Self::remove_all_outward_adjacencies_for_face(
                    voxels,
                    upper_chunk_data_offset,
                    dim,
                    Side::Lower,
                );

                split_detector.remove_connections_for_non_uniform_chunk(
                    upper_chunk_data_offset,
                    upper_chunk_split_detection,
                    dim,
                    Side::Lower,
                );

                upper_chunk.mark_lower_face_as_unobscured(dim);
            }
            // If one is non-uniform and the other is empty, we need to clear
            // the adjacencies of the non-uniform chunk with the empty chunk, as
            // well as mark the adjoining face of the non-uniform chunk as
            // unobscured
            (
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: lower_chunk_data_offset,
                    face_distributions: lower_chunk_face_distributions,
                    split_detection: lower_chunk_split_detection,
                    ..
                }),
                Self::Empty,
            ) => {
                // We can skip this update if there are no voxels on the face
                if lower_chunk_face_distributions[dim.idx()][1] != FaceVoxelDistribution::Empty {
                    Self::remove_all_outward_adjacencies_for_face(
                        voxels,
                        lower_chunk_data_offset,
                        dim,
                        Side::Upper,
                    );

                    split_detector.remove_connections_for_non_uniform_chunk(
                        lower_chunk_data_offset,
                        lower_chunk_split_detection,
                        dim,
                        Side::Upper,
                    );
                }

                chunks[lower_chunk_idx.unwrap()].mark_upper_face_as_unobscured(dim);
            }
            (
                Self::Empty,
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: upper_chunk_data_offset,
                    face_distributions: upper_chunk_face_distributions,
                    split_detection: upper_chunk_split_detection,
                    ..
                }),
            ) => {
                if upper_chunk_face_distributions[dim.idx()][0] != FaceVoxelDistribution::Empty {
                    Self::remove_all_outward_adjacencies_for_face(
                        voxels,
                        upper_chunk_data_offset,
                        dim,
                        Side::Lower,
                    );

                    split_detector.remove_connections_for_non_uniform_chunk(
                        upper_chunk_data_offset,
                        upper_chunk_split_detection,
                        dim,
                        Side::Lower,
                    );
                }

                chunks[upper_chunk_idx.unwrap()].mark_lower_face_as_unobscured(dim);
            }
            // If one is non-uniform and the other is uniform, we need to set
            // the adjacencies of the non-uniform chunk with the uniform chunk,
            // and if the adjoining face of the non-uniform chunk is not full,
            // we must convert the uniform chunk to non-uniform and update its
            // adjacencies as well. We also need to mark the adjoining face of
            // the non-uniform chunk as obscured, and potentially the adjoining
            // face of the uniform one as unobscured.
            (
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: lower_chunk_data_offset,
                    face_distributions: lower_chunk_face_distributions,
                    split_detection: lower_chunk_split_detection,
                    ..
                }),
                Self::Uniform(UniformVoxelChunk {
                    split_detection: upper_chunk_split_detection,
                    ..
                }),
            ) => {
                let lower_chunk_face_distribution = lower_chunk_face_distributions[dim.idx()][1];

                if lower_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    Self::add_all_outward_adjacencies_for_face(
                        voxels,
                        lower_chunk_data_offset,
                        dim,
                        Side::Upper,
                    );

                    split_detector.update_connections_from_non_uniform_chunk_to_uniform_chunk(
                        lower_chunk_data_offset,
                        lower_chunk_split_detection,
                        dim,
                        Side::Upper,
                    );
                }

                chunks[lower_chunk_idx.unwrap()].mark_upper_face_as_obscured(dim);

                match lower_chunk_face_distribution {
                    FaceVoxelDistribution::Full => {
                        split_detector.update_connections_from_uniform_chunk_to_non_uniform_chunk(
                            upper_chunk_split_detection,
                            lower_chunk_data_offset,
                            dim,
                            Side::Lower,
                        );
                    }
                    FaceVoxelDistribution::Empty => {
                        let upper_chunk = &mut chunks[upper_chunk_idx.unwrap()];
                        upper_chunk.convert_to_non_uniform_if_uniform(voxels, split_detector);

                        let (upper_chunk_data_offset, upper_chunk_split_detection) = upper_chunk
                            .data_offset_and_split_detection_if_non_uniform()
                            .unwrap();

                        Self::remove_all_outward_adjacencies_for_face(
                            voxels,
                            upper_chunk_data_offset,
                            dim,
                            Side::Lower,
                        );

                        split_detector.remove_connections_for_non_uniform_chunk(
                            upper_chunk_data_offset,
                            upper_chunk_split_detection,
                            dim,
                            Side::Lower,
                        );

                        upper_chunk.mark_lower_face_as_unobscured(dim);
                    }
                    FaceVoxelDistribution::Mixed => {
                        let upper_chunk = &mut chunks[upper_chunk_idx.unwrap()];
                        upper_chunk.convert_to_non_uniform_if_uniform(voxels, split_detector);

                        let (upper_chunk_data_offset, upper_chunk_split_detection) = upper_chunk
                            .data_offset_and_split_detection_if_non_uniform()
                            .unwrap();

                        Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                            voxels,
                            split_detector,
                            upper_chunk_data_offset,
                            lower_chunk_data_offset,
                            upper_chunk_split_detection,
                            dim,
                            Side::Lower,
                        );

                        upper_chunk.mark_lower_face_as_unobscured(dim);
                    }
                }
            }
            (
                Self::Uniform(UniformVoxelChunk {
                    split_detection: lower_chunk_split_detection,
                    ..
                }),
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: upper_chunk_data_offset,
                    face_distributions: upper_chunk_face_distributions,
                    split_detection: upper_chunk_split_detection,
                    ..
                }),
            ) => {
                let upper_chunk_face_distribution = upper_chunk_face_distributions[dim.idx()][0];

                if upper_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    Self::add_all_outward_adjacencies_for_face(
                        voxels,
                        upper_chunk_data_offset,
                        dim,
                        Side::Lower,
                    );

                    split_detector.update_connections_from_non_uniform_chunk_to_uniform_chunk(
                        upper_chunk_data_offset,
                        upper_chunk_split_detection,
                        dim,
                        Side::Lower,
                    );
                }

                chunks[upper_chunk_idx.unwrap()].mark_lower_face_as_obscured(dim);

                match upper_chunk_face_distribution {
                    FaceVoxelDistribution::Full => {
                        split_detector.update_connections_from_uniform_chunk_to_non_uniform_chunk(
                            lower_chunk_split_detection,
                            upper_chunk_data_offset,
                            dim,
                            Side::Upper,
                        );
                    }
                    FaceVoxelDistribution::Empty => {
                        let lower_chunk = &mut chunks[lower_chunk_idx.unwrap()];
                        lower_chunk.convert_to_non_uniform_if_uniform(voxels, split_detector);

                        let (lower_chunk_data_offset, lower_chunk_split_detection) = lower_chunk
                            .data_offset_and_split_detection_if_non_uniform()
                            .unwrap();

                        Self::remove_all_outward_adjacencies_for_face(
                            voxels,
                            lower_chunk_data_offset,
                            dim,
                            Side::Upper,
                        );

                        split_detector.remove_connections_for_non_uniform_chunk(
                            lower_chunk_data_offset,
                            lower_chunk_split_detection,
                            dim,
                            Side::Upper,
                        );

                        lower_chunk.mark_upper_face_as_unobscured(dim);
                    }
                    FaceVoxelDistribution::Mixed => {
                        let lower_chunk = &mut chunks[lower_chunk_idx.unwrap()];
                        lower_chunk.convert_to_non_uniform_if_uniform(voxels, split_detector);

                        let (lower_chunk_data_offset, lower_chunk_split_detection) = lower_chunk
                            .data_offset_and_split_detection_if_non_uniform()
                            .unwrap();

                        Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                            voxels,
                            split_detector,
                            lower_chunk_data_offset,
                            upper_chunk_data_offset,
                            lower_chunk_split_detection,
                            dim,
                            Side::Upper,
                        );

                        lower_chunk.mark_upper_face_as_unobscured(dim);
                    }
                }
            }
            // If both chunks are non-uniform, we need to update the adjacencies
            // and obscuredness for both according to their adjoining faces
            (
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: lower_chunk_data_offset,
                    face_distributions: lower_chunk_face_distributions,
                    split_detection: lower_chunk_split_detection,
                    ..
                }),
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: upper_chunk_data_offset,
                    face_distributions: upper_chunk_face_distributions,
                    split_detection: upper_chunk_split_detection,
                    ..
                }),
            ) => {
                let lower_chunk_face_distribution = lower_chunk_face_distributions[dim.idx()][1];
                let upper_chunk_face_distribution = upper_chunk_face_distributions[dim.idx()][0];

                if lower_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    match upper_chunk_face_distribution {
                        FaceVoxelDistribution::Empty => {
                            Self::remove_all_outward_adjacencies_for_face(
                                voxels,
                                lower_chunk_data_offset,
                                dim,
                                Side::Upper,
                            );
                            split_detector.remove_connections_for_non_uniform_chunk(
                                lower_chunk_data_offset,
                                lower_chunk_split_detection,
                                dim,
                                Side::Upper,
                            );
                        }
                        FaceVoxelDistribution::Full => {
                            Self::add_all_outward_adjacencies_for_face(
                                voxels,
                                lower_chunk_data_offset,
                                dim,
                                Side::Upper,
                            );
                            split_detector
                                .update_connections_from_non_uniform_chunk_to_non_uniform_chunk_with_full_face(
                                    lower_chunk_data_offset,
                                    lower_chunk_split_detection,
                                    upper_chunk_data_offset,
                                    dim,
                                    Side::Upper,
                                );
                        }
                        FaceVoxelDistribution::Mixed => {
                            Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                                voxels,
                                split_detector,
                                lower_chunk_data_offset,
                                upper_chunk_data_offset,
                                lower_chunk_split_detection,
                               dim, Side::Upper,
                            );
                        }
                    }
                }

                if upper_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    match lower_chunk_face_distribution {
                        FaceVoxelDistribution::Empty => {
                            Self::remove_all_outward_adjacencies_for_face(
                                voxels,
                                upper_chunk_data_offset,
                                dim,
                                Side::Lower,
                            );
                            split_detector.remove_connections_for_non_uniform_chunk(
                                upper_chunk_data_offset,
                                upper_chunk_split_detection,
                                dim,
                                Side::Lower,
                            );
                        }
                        FaceVoxelDistribution::Full => {
                            Self::add_all_outward_adjacencies_for_face(
                                voxels,
                                upper_chunk_data_offset,
                                dim,
                                Side::Lower,
                            );
                            split_detector
                                .update_connections_from_non_uniform_chunk_to_non_uniform_chunk_with_full_face(
                                    upper_chunk_data_offset,
                                    upper_chunk_split_detection,
                                    lower_chunk_data_offset,
                                    dim,
                                    Side::Lower,
                                );
                        }
                        FaceVoxelDistribution::Mixed => {
                            Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                                voxels,
                                split_detector,
                                upper_chunk_data_offset,
                                lower_chunk_data_offset,
                                upper_chunk_split_detection,
                               dim, Side::Lower,
                            );
                        }
                    }
                }

                let lower_chunk = &mut chunks[lower_chunk_idx.unwrap()];
                if upper_chunk_face_distribution == FaceVoxelDistribution::Full {
                    lower_chunk.mark_upper_face_as_obscured(dim);
                } else {
                    lower_chunk.mark_upper_face_as_unobscured(dim);
                }

                let upper_chunk = &mut chunks[upper_chunk_idx.unwrap()];
                if lower_chunk_face_distribution == FaceVoxelDistribution::Full {
                    upper_chunk.mark_lower_face_as_obscured(dim);
                } else {
                    upper_chunk.mark_lower_face_as_unobscured(dim);
                }
            }
        }
    }

    fn convert_to_non_uniform_if_uniform(
        &mut self,
        voxels: &mut Vec<Voxel>,
        split_detector: &mut SplitDetector,
    ) {
        if let &mut Self::Uniform(UniformVoxelChunk {
            voxel,
            split_detection,
        }) = self
        {
            let start_voxel_idx = voxels.len();
            voxels.reserve(CHUNK_VOXEL_COUNT);
            voxels.extend(iter::repeat_n(voxel, CHUNK_VOXEL_COUNT));
            *self = Self::NonUniform(NonUniformVoxelChunk {
                data_offset: chunk_data_offset_from_start_voxel_idx(start_voxel_idx),
                face_distributions: [[FaceVoxelDistribution::Full; 2]; 3],
                flags: VoxelChunkFlags::fully_obscured(),
                split_detection: NonUniformChunkSplitDetectionData::for_previously_uniform(),
            });
            split_detector.convert_uniform_chunk_to_non_uniform(split_detection);
        }
    }

    fn add_all_outward_adjacencies_for_face(
        voxels: &mut [Voxel],
        data_offset: u32,
        face_dim: Dimension,
        face_side: Side,
    ) {
        Self::update_all_outward_adjacencies_for_face(
            voxels,
            data_offset,
            face_dim,
            face_side,
            &Voxel::add_flags,
        );
    }

    fn remove_all_outward_adjacencies_for_face(
        voxels: &mut [Voxel],
        data_offset: u32,
        face_dim: Dimension,
        face_side: Side,
    ) {
        Self::update_all_outward_adjacencies_for_face(
            voxels,
            data_offset,
            face_dim,
            face_side,
            &Voxel::remove_flags,
        );
    }

    fn update_all_outward_adjacencies_for_face(
        voxels: &mut [Voxel],
        data_offset: u32,
        face_dim: Dimension,
        face_side: Side,
        update_flags: &impl Fn(&mut Voxel, VoxelFlags),
    ) {
        let chunk_voxels = chunk_voxels_mut(voxels, data_offset);

        let flag = VoxelFlags::adjacency_for_face(face_dim, face_side);

        LoopOverChunkVoxelDataMut::new(
            &LoopForChunkVoxels::over_face(face_dim, face_side),
            chunk_voxels,
        )
        .execute(&mut |_, voxel| {
            update_flags(voxel, flag);
        });
    }

    fn update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
        voxels: &mut [Voxel],
        split_detector: &mut SplitDetector,
        current_chunk_data_offset: u32,
        adjacent_chunk_data_offset: u32,
        current_chunk_split_detection: NonUniformChunkSplitDetectionData,
        face_dim: Dimension,
        face_side: Side,
    ) {
        let current_chunk_start_voxel_idx = chunk_start_voxel_idx(current_chunk_data_offset);
        let adjacent_chunk_start_voxel_idx = chunk_start_voxel_idx(adjacent_chunk_data_offset);

        let (current_chunk_voxels, adjacent_chunk_voxels) = extract_slice_segments_mut(
            voxels,
            current_chunk_start_voxel_idx,
            adjacent_chunk_start_voxel_idx,
            CHUNK_VOXEL_COUNT,
        );

        let flag = VoxelFlags::adjacency_for_face(face_dim, face_side);

        let mut split_updater = split_detector.begin_non_uniform_chunk_connection_update(
            current_chunk_data_offset,
            adjacent_chunk_data_offset,
            current_chunk_split_detection,
            face_dim,
            face_side,
        );

        LoopForChunkVoxels::over_face(face_dim, face_side).zip_execute(
            &LoopForChunkVoxels::over_face(face_dim, face_side.opposite()),
            &mut |current_indices, adjacent_indices| {
                let current_chunk_voxel_idx = linear_voxel_idx_within_chunk(current_indices);
                let current_chunk_voxel = &mut current_chunk_voxels[current_chunk_voxel_idx];

                if !current_chunk_voxel.is_empty() {
                    let adjacent_chunk_voxel_idx = linear_voxel_idx_within_chunk(adjacent_indices);
                    if adjacent_chunk_voxels[adjacent_chunk_voxel_idx].is_empty() {
                        current_chunk_voxel.remove_flags(flag);
                    } else {
                        current_chunk_voxel.add_flags(flag);

                        split_updater.update_for_non_empty_adjacent_voxel(
                            current_chunk_voxel_idx,
                            adjacent_chunk_voxel_idx,
                        );
                    }
                }
            },
        );
    }
}

impl NonUniformVoxelChunk {
    #[inline]
    const fn contains_only_empty_voxels(&self) -> bool {
        self.flags.contains(VoxelChunkFlags::IS_EMPTY)
    }

    fn update_internal_adjacencies(&self, voxels: &mut [Voxel]) {
        // Extract the sub-slice of voxels for this chunk so that we get
        // out-of-bounds if trying to access voxels outside the chunk
        let chunk_voxels = chunk_voxels_mut(voxels, self.data_offset);

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let idx = linear_voxel_idx_within_chunk(&[i, j, k]);

                    let voxel = chunk_voxels[idx];

                    if voxel.is_empty() {
                        // Since we will update the flag of the adjacent voxel in
                        // addition to this one, we only need to look up the upper
                        // adjacent voxels to cover every adjacency over the course
                        // of the full loop
                        for (adjacent_indices, flag_for_adjacent, dim) in [
                            ([i + 1, j, k], VoxelFlags::HAS_ADJACENT_X_DN, Dimension::X),
                            ([i, j + 1, k], VoxelFlags::HAS_ADJACENT_Y_DN, Dimension::Y),
                            ([i, j, k + 1], VoxelFlags::HAS_ADJACENT_Z_DN, Dimension::Z),
                        ] {
                            if adjacent_indices[dim.idx()] < CHUNK_SIZE {
                                let adjacent_idx = linear_voxel_idx_within_chunk(&adjacent_indices);
                                cfg_if! {
                                    if #[cfg(feature = "unchecked")] {
                                        let adjacent_voxel =
                                            unsafe { chunk_voxels.get_unchecked_mut(adjacent_idx) };
                                    } else {
                                        let adjacent_voxel = &mut chunk_voxels[adjacent_idx];
                                    }
                                }
                                adjacent_voxel.remove_flags(flag_for_adjacent);
                            }
                        }
                    } else {
                        let mut flags = voxel.flags();

                        for (adjacent_indices, flag_for_current, flag_for_adjacent, dim) in [
                            (
                                [i + 1, j, k],
                                VoxelFlags::HAS_ADJACENT_X_UP,
                                VoxelFlags::HAS_ADJACENT_X_DN,
                                Dimension::X,
                            ),
                            (
                                [i, j + 1, k],
                                VoxelFlags::HAS_ADJACENT_Y_UP,
                                VoxelFlags::HAS_ADJACENT_Y_DN,
                                Dimension::Y,
                            ),
                            (
                                [i, j, k + 1],
                                VoxelFlags::HAS_ADJACENT_Z_UP,
                                VoxelFlags::HAS_ADJACENT_Z_DN,
                                Dimension::Z,
                            ),
                        ] {
                            if adjacent_indices[dim.idx()] < CHUNK_SIZE {
                                let adjacent_idx = linear_voxel_idx_within_chunk(&adjacent_indices);
                                cfg_if! {
                                    if #[cfg(feature = "unchecked")] {
                                        let adjacent_voxel =
                                            unsafe { chunk_voxels.get_unchecked_mut(adjacent_idx) };
                                    } else {
                                        let adjacent_voxel = &mut chunk_voxels[adjacent_idx];
                                    }
                                }
                                if adjacent_voxel.is_empty() {
                                    flags -= flag_for_current;
                                } else {
                                    flags |= flag_for_current;
                                    adjacent_voxel.add_flags(flag_for_adjacent);
                                }
                            }
                        }

                        chunk_voxels[idx].update_flags(flags);
                    }
                }
            }
        }
    }

    fn update_face_distributions_and_internal_adjacencies_and_count_non_empty_voxels(
        &mut self,
        voxels: &mut [Voxel],
    ) -> usize {
        // Extract the sub-slice of voxels for this chunk so that we get
        // out-of-bounds if trying to access voxels outside the chunk
        let chunk_voxels = chunk_voxels_mut(voxels, self.data_offset);

        let mut face_empty_counts = FaceEmptyCounts::zero();
        let mut non_empty_voxel_count = 0;

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let idx = linear_voxel_idx_within_chunk(&[i, j, k]);

                    let voxel = chunk_voxels[idx];

                    if voxel.is_empty() {
                        if i == 0 {
                            face_empty_counts.increment_x_dn();
                        } else if i == CHUNK_SIZE - 1 {
                            face_empty_counts.increment_x_up();
                        }
                        if j == 0 {
                            face_empty_counts.increment_y_dn();
                        } else if j == CHUNK_SIZE - 1 {
                            face_empty_counts.increment_y_up();
                        }
                        if k == 0 {
                            face_empty_counts.increment_z_dn();
                        } else if k == CHUNK_SIZE - 1 {
                            face_empty_counts.increment_z_up();
                        }

                        // Since we will update the flag of the adjacent voxel in
                        // addition to this one, we only need to look up the upper
                        // adjacent voxels to cover every adjacency over the course
                        // of the full loop
                        for (adjacent_indices, flag_for_adjacent, dim) in [
                            ([i + 1, j, k], VoxelFlags::HAS_ADJACENT_X_DN, Dimension::X),
                            ([i, j + 1, k], VoxelFlags::HAS_ADJACENT_Y_DN, Dimension::Y),
                            ([i, j, k + 1], VoxelFlags::HAS_ADJACENT_Z_DN, Dimension::Z),
                        ] {
                            if adjacent_indices[dim.idx()] < CHUNK_SIZE {
                                let adjacent_idx = linear_voxel_idx_within_chunk(&adjacent_indices);
                                cfg_if! {
                                    if #[cfg(feature = "unchecked")] {
                                        let adjacent_voxel =
                                            unsafe { chunk_voxels.get_unchecked_mut(adjacent_idx) };
                                    } else {
                                        let adjacent_voxel = &mut chunk_voxels[adjacent_idx];
                                    }
                                }
                                adjacent_voxel.remove_flags(flag_for_adjacent);
                            }
                        }
                    } else {
                        let mut flags = voxel.flags();

                        for (adjacent_indices, flag_for_current, flag_for_adjacent, dim) in [
                            (
                                [i + 1, j, k],
                                VoxelFlags::HAS_ADJACENT_X_UP,
                                VoxelFlags::HAS_ADJACENT_X_DN,
                                Dimension::X,
                            ),
                            (
                                [i, j + 1, k],
                                VoxelFlags::HAS_ADJACENT_Y_UP,
                                VoxelFlags::HAS_ADJACENT_Y_DN,
                                Dimension::Y,
                            ),
                            (
                                [i, j, k + 1],
                                VoxelFlags::HAS_ADJACENT_Z_UP,
                                VoxelFlags::HAS_ADJACENT_Z_DN,
                                Dimension::Z,
                            ),
                        ] {
                            if adjacent_indices[dim.idx()] < CHUNK_SIZE {
                                let adjacent_idx = linear_voxel_idx_within_chunk(&adjacent_indices);
                                cfg_if! {
                                    if #[cfg(feature = "unchecked")] {
                                        let adjacent_voxel =
                                            unsafe { chunk_voxels.get_unchecked_mut(adjacent_idx) };
                                    } else {
                                        let adjacent_voxel = &mut chunk_voxels[adjacent_idx];
                                    }
                                }
                                if adjacent_voxel.is_empty() {
                                    flags -= flag_for_current;
                                } else {
                                    flags |= flag_for_current;
                                    adjacent_voxel.add_flags(flag_for_adjacent);
                                }
                            }
                        }

                        chunk_voxels[idx].update_flags(flags);
                        non_empty_voxel_count += 1;
                    }
                }
            }
        }

        self.face_distributions = face_empty_counts.to_chunk_face_distributions();

        non_empty_voxel_count
    }
}

impl FaceEmptyCounts {
    #[inline]
    const fn zero() -> Self {
        Self([[0; 2]; 3])
    }

    #[inline]
    fn increment_x_dn(&mut self) {
        self.0[0][0] += 1;
    }
    #[inline]
    fn increment_x_up(&mut self) {
        self.0[0][1] += 1;
    }
    #[inline]
    fn increment_y_dn(&mut self) {
        self.0[1][0] += 1;
    }
    #[inline]
    fn increment_y_up(&mut self) {
        self.0[1][1] += 1;
    }
    #[inline]
    fn increment_z_dn(&mut self) {
        self.0[2][0] += 1;
    }
    #[inline]
    fn increment_z_up(&mut self) {
        self.0[2][1] += 1;
    }

    #[inline]
    fn to_chunk_face_distributions(&self) -> [[FaceVoxelDistribution; 2]; 3] {
        self.to_face_distributions(CHUNK_SIZE_SQUARED)
    }

    #[inline]
    fn to_face_distributions(&self, full_face_count: usize) -> [[FaceVoxelDistribution; 2]; 3] {
        self.map(&|empty_count| {
            if empty_count == full_face_count {
                FaceVoxelDistribution::Empty
            } else if empty_count == 0 {
                FaceVoxelDistribution::Full
            } else {
                FaceVoxelDistribution::Mixed
            }
        })
    }

    #[inline]
    fn map<T>(&self, f: &impl Fn(usize) -> T) -> [[T; 2]; 3] {
        self.0.map(|counts| counts.map(f))
    }
}

impl VoxelChunkFlags {
    #[inline]
    const fn fully_obscured() -> Self {
        Self::IS_OBSCURED_X_DN
            .union(Self::IS_OBSCURED_Y_DN)
            .union(Self::IS_OBSCURED_Z_DN)
            .union(Self::IS_OBSCURED_X_UP)
            .union(Self::IS_OBSCURED_Y_UP)
            .union(Self::IS_OBSCURED_Z_UP)
    }

    #[inline]
    fn has_exposed_face(&self) -> bool {
        !self.contains(Self::fully_obscured())
    }

    #[inline]
    fn mark_lower_face_as_obscured(&mut self, dim: Dimension) {
        self.insert(Self::from_bits_retain(1 << dim as u8));
    }

    #[inline]
    fn mark_upper_face_as_obscured(&mut self, dim: Dimension) {
        self.insert(Self::from_bits_retain(1 << (3 + dim as u8)));
    }

    #[inline]
    fn mark_lower_face_as_unobscured(&mut self, dim: Dimension) {
        self.remove(Self::from_bits_retain(1 << dim as u8));
    }

    #[inline]
    fn mark_upper_face_as_unobscured(&mut self, dim: Dimension) {
        self.remove(Self::from_bits_retain(1 << (3 + dim as u8)));
    }
}

impl ExposedVoxelChunk {
    #[inline]
    fn new(chunk_indices: [usize; 3], flags: VoxelChunkFlags) -> Self {
        Self {
            chunk_indices,
            flags,
        }
    }

    /// Returns the indices of the voxel chunk in the object's chunk grid.
    #[inline]
    pub fn chunk_indices(&self) -> &[usize; 3] {
        &self.chunk_indices
    }

    /// Returns the flags for the voxel chunk.
    #[inline]
    pub fn flags(&self) -> VoxelChunkFlags {
        self.flags
    }

    #[inline]
    pub fn lower_voxel_indices(&self) -> [usize; 3] {
        [
            self.chunk_indices[0] * CHUNK_SIZE,
            self.chunk_indices[1] * CHUNK_SIZE,
            self.chunk_indices[2] * CHUNK_SIZE,
        ]
    }

    #[inline]
    pub fn upper_voxel_indices(&self) -> [usize; 3] {
        [
            self.chunk_indices[0] * CHUNK_SIZE + CHUNK_SIZE - 1,
            self.chunk_indices[1] * CHUNK_SIZE + CHUNK_SIZE - 1,
            self.chunk_indices[2] * CHUNK_SIZE + CHUNK_SIZE - 1,
        ]
    }
}

fn determine_occupied_voxel_ranges(
    chunk_counts: [usize; 3],
    chunks: &[VoxelChunk],
    voxels: &[Voxel],
) -> [Range<usize>; 3] {
    let mut min_voxel_indices = [usize::MAX; 3];
    let mut max_voxel_indices = [0; 3];

    let mut chunk_idx = 0;
    for chunk_i in 0..chunk_counts[0] {
        for chunk_j in 0..chunk_counts[1] {
            for chunk_k in 0..chunk_counts[2] {
                let voxel_index_offsets = [
                    chunk_i * CHUNK_SIZE,
                    chunk_j * CHUNK_SIZE,
                    chunk_k * CHUNK_SIZE,
                ];
                match &chunks[chunk_idx] {
                    VoxelChunk::NonUniform(NonUniformVoxelChunk { data_offset, .. }) => {
                        let chunk_voxels = chunk_voxels(voxels, *data_offset);
                        LoopOverChunkVoxelData::new(&LoopForChunkVoxels::over_all(), chunk_voxels)
                            .execute(&mut |&voxel_indices, voxel| {
                                if !voxel.is_empty() {
                                    let object_voxel_indices = array::from_fn(|idx| {
                                        voxel_index_offsets[idx] + voxel_indices[idx]
                                    });

                                    min_voxel_indices = componentwise_min_indices(
                                        &min_voxel_indices,
                                        &object_voxel_indices,
                                    );
                                    max_voxel_indices = componentwise_max_indices(
                                        &max_voxel_indices,
                                        &object_voxel_indices,
                                    );
                                }
                            });
                    }
                    VoxelChunk::Uniform(_) => {
                        min_voxel_indices =
                            componentwise_min_indices(&min_voxel_indices, &voxel_index_offsets);

                        max_voxel_indices = componentwise_max_indices(
                            &max_voxel_indices,
                            &voxel_index_offsets.map(|offset| offset + (CHUNK_SIZE - 1)),
                        );
                    }
                    VoxelChunk::Empty => {}
                }
                chunk_idx += 1;
            }
        }
    }

    array::from_fn(|dim| min_voxel_indices[dim]..max_voxel_indices[dim] + 1)
}

#[inline]
fn chunk_indices_from_linear_idx(chunk_counts: &[usize; 3], chunk_idx: usize) -> [usize; 3] {
    let chunk_i = chunk_idx / (chunk_counts[2] * chunk_counts[1]);
    let chunk_j = (chunk_idx / chunk_counts[2]) % chunk_counts[1];
    let chunk_k = chunk_idx % chunk_counts[2];
    [chunk_i, chunk_j, chunk_k]
}

#[inline]
const fn chunk_start_voxel_idx(data_offset: u32) -> usize {
    (data_offset as usize) << (3 * LOG2_CHUNK_SIZE)
}

#[inline]
const fn chunk_data_offset_from_start_voxel_idx(start_voxel_idx: usize) -> u32 {
    (start_voxel_idx >> (3 * LOG2_CHUNK_SIZE)) as u32
}

#[inline]
fn chunk_voxels(voxels: &[Voxel], data_offset: u32) -> &[Voxel] {
    let start_voxel_idx = chunk_start_voxel_idx(data_offset);
    &voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
}

#[inline]
fn chunk_voxels_mut(voxels: &mut [Voxel], data_offset: u32) -> &mut [Voxel] {
    let start_voxel_idx = chunk_start_voxel_idx(data_offset);
    &mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
}

/// Computes the index into a chunk's flattened voxel grid of the voxel at the
/// given indices in the parent object's voxel grid.
#[inline]
const fn linear_voxel_idx_within_chunk_from_object_voxel_indices(
    i: usize,
    j: usize,
    k: usize,
) -> usize {
    let voxel_indices = voxel_indices_within_chunk_from_object_voxel_indices(i, j, k);
    linear_voxel_idx_within_chunk(&voxel_indices)
}

/// Computes the index into a chunk's flattened voxel grid of the voxel with the
/// given 3D index in the voxel grid.
#[inline]
const fn linear_voxel_idx_within_chunk(voxel_indices: &[usize; 3]) -> usize {
    (voxel_indices[0] << (2 * LOG2_CHUNK_SIZE))
        + (voxel_indices[1] << LOG2_CHUNK_SIZE)
        + voxel_indices[2]
}

/// Computes the 3D index into a chunk's voxel grid for the voxel with the
/// given linear index into the flattened version of the chunk's voxel grid.
#[inline]
const fn chunk_voxel_indices_from_linear_idx(idx: usize) -> [usize; 3] {
    [
        idx >> (2 * LOG2_CHUNK_SIZE),
        (idx >> LOG2_CHUNK_SIZE) & VOXEL_INDEX_FROM_LINEAR_IDX_MASK,
        idx & VOXEL_INDEX_FROM_LINEAR_IDX_MASK,
    ]
}

/// Computes the 3D index in the parent object's chunk grid of the chunk
/// containing the voxel at the given indices in the object's voxel grid.
///
/// Since chunks have a power-of-two number of voxels along each axis, the
/// chunk index is encoded in the upper bits of the corresponding object voxel
/// index.
#[inline]
const fn chunk_indices_from_object_voxel_indices(i: usize, j: usize, k: usize) -> [usize; 3] {
    [
        i >> CHUNK_IDX_FROM_OBJECT_VOXEL_IDX_SHIFT,
        j >> CHUNK_IDX_FROM_OBJECT_VOXEL_IDX_SHIFT,
        k >> CHUNK_IDX_FROM_OBJECT_VOXEL_IDX_SHIFT,
    ]
}

/// Computes the 3D index in a chunk's voxel grid of the voxel at the given
/// indices in the parent object's voxel grid.
///
/// Since chunks have a power-of-two number of voxels along each axis, the voxel
/// index within the chunk is encoded in the lower bits of the corresponding
/// object voxel index.
#[inline]
const fn voxel_indices_within_chunk_from_object_voxel_indices(
    i: usize,
    j: usize,
    k: usize,
) -> [usize; 3] {
    [
        i & VOXEL_IDX_FROM_OBJECT_VOXEL_IDX_MASK,
        j & VOXEL_IDX_FROM_OBJECT_VOXEL_IDX_MASK,
        k & VOXEL_IDX_FROM_OBJECT_VOXEL_IDX_MASK,
    ]
}

fn extract_slice_segments_mut<T>(
    slice: &mut [T],
    segment_1_start_idx: usize,
    segment_2_start_idx: usize,
    segment_len: usize,
) -> (&mut [T], &mut [T]) {
    assert_ne!(segment_1_start_idx, segment_2_start_idx);

    let (values_before_1, values_from_1) = slice.split_at_mut(segment_1_start_idx);

    let (values_from_1, values_from_2) = if segment_2_start_idx > segment_1_start_idx {
        values_from_1.split_at_mut(segment_2_start_idx - segment_1_start_idx)
    } else {
        let (_, values_from_2) = values_before_1.split_at_mut(segment_2_start_idx);
        (values_from_1, values_from_2)
    };

    (
        &mut values_from_1[..segment_len],
        &mut values_from_2[..segment_len],
    )
}

#[inline]
fn componentwise_min_indices(a: &[usize; 3], b: &[usize; 3]) -> [usize; 3] {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])]
}

#[inline]
fn componentwise_max_indices(a: &[usize; 3], b: &[usize; 3]) -> [usize; 3] {
    [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])]
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::generation::SDFVoxelGenerator;
    use impact_alloc::Global;

    pub fn fuzz_test_voxel_object_generation(generator: SDFVoxelGenerator<Global>) {
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_occupied_voxel_ranges();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
        object.validate_sdf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel_types::VoxelType;
    use approx::assert_abs_diff_eq;
    use impact_alloc::Allocator;

    pub struct OffsetBoxVoxelGenerator {
        shape: [usize; 3],
        offset: [usize; 3],
        voxel: Voxel,
    }

    struct ManualVoxelGenerator<const N: usize> {
        voxels: [[[u8; N]; N]; N],
        offset: [usize; 3],
    }

    impl OffsetBoxVoxelGenerator {
        pub fn new(shape: [usize; 3], offset: [usize; 3], voxel: Voxel) -> Self {
            Self {
                shape,
                offset,
                voxel,
            }
        }

        pub fn empty(shape: [usize; 3]) -> Self {
            Self::new(shape, [0; 3], Voxel::maximally_outside())
        }

        pub fn single(voxel: Voxel) -> Self {
            Self::new([1, 1, 1], [0; 3], voxel)
        }

        pub fn single_default() -> Self {
            Self::single(Voxel::maximally_inside(VoxelType::default()))
        }

        pub fn single_empty() -> Self {
            Self::single(Voxel::maximally_outside())
        }

        pub fn with_default(shape: [usize; 3]) -> Self {
            Self::offset_with_default(shape, [0; 3])
        }

        pub fn offset_with_default(shape: [usize; 3], offset: [usize; 3]) -> Self {
            Self::new(shape, offset, Voxel::maximally_inside(VoxelType::default()))
        }
    }

    impl<const N: usize> ManualVoxelGenerator<N> {
        fn new(voxels: [[[u8; N]; N]; N]) -> Self {
            Self::with_offset(voxels, [0; 3])
        }

        fn with_offset(voxels: [[[u8; N]; N]; N], offset: [usize; 3]) -> Self {
            Self { voxels, offset }
        }
    }

    impl ChunkedVoxelGenerator for OffsetBoxVoxelGenerator {
        type ChunkGenerationBuffers<AB: Allocator> = ();

        fn voxel_extent(&self) -> f32 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            [
                self.offset[0] + self.shape[0],
                self.offset[1] + self.shape[1],
                self.offset[2] + self.shape[2],
            ]
        }

        fn total_buffer_size(&self) -> usize {
            0
        }

        fn create_buffers_in<AB: Allocator>(&self, _alloc: AB) -> Self::ChunkGenerationBuffers<AB> {
        }

        fn generate_chunk<AB: Allocator>(
            &self,
            _buffers: &mut Self::ChunkGenerationBuffers<AB>,
            voxels: &mut [Voxel],
            chunk_origin: &[usize; 3],
        ) {
            assert_eq!(voxels.len(), CHUNK_VOXEL_COUNT);
            let mut idx = 0;
            for i in chunk_origin[0]..chunk_origin[0] + CHUNK_SIZE {
                for j in chunk_origin[1]..chunk_origin[1] + CHUNK_SIZE {
                    for k in chunk_origin[2]..chunk_origin[2] + CHUNK_SIZE {
                        let voxel = if i >= self.offset[0]
                            && i < self.offset[0] + self.shape[0]
                            && j >= self.offset[1]
                            && j < self.offset[1] + self.shape[1]
                            && k >= self.offset[2]
                            && k < self.offset[2] + self.shape[2]
                        {
                            self.voxel
                        } else {
                            Voxel::maximally_outside()
                        };
                        voxels[idx] = voxel;
                        idx += 1;
                    }
                }
            }
        }
    }

    impl<const N: usize> ChunkedVoxelGenerator for ManualVoxelGenerator<N> {
        type ChunkGenerationBuffers<AB: Allocator> = ();

        fn voxel_extent(&self) -> f32 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            [self.offset[0] + N, self.offset[1] + N, self.offset[2] + N]
        }

        fn total_buffer_size(&self) -> usize {
            0
        }

        fn create_buffers_in<AB: Allocator>(&self, _alloc: AB) -> Self::ChunkGenerationBuffers<AB> {
        }

        fn generate_chunk<AB: Allocator>(
            &self,
            _buffers: &mut Self::ChunkGenerationBuffers<AB>,
            voxels: &mut [Voxel],
            chunk_origin: &[usize; 3],
        ) {
            assert_eq!(voxels.len(), CHUNK_VOXEL_COUNT);
            let mut idx = 0;
            for i in chunk_origin[0]..chunk_origin[0] + CHUNK_SIZE {
                for j in chunk_origin[1]..chunk_origin[1] + CHUNK_SIZE {
                    for k in chunk_origin[2]..chunk_origin[2] + CHUNK_SIZE {
                        let voxel = if i >= self.offset[0]
                            && i < self.offset[0] + N
                            && j >= self.offset[1]
                            && j < self.offset[1] + N
                            && k >= self.offset[2]
                            && k < self.offset[2] + N
                            && self.voxels[i - self.offset[0]][j - self.offset[1]]
                                [k - self.offset[2]]
                                != 0
                        {
                            Voxel::maximally_inside(VoxelType::default())
                        } else {
                            Voxel::maximally_outside()
                        };
                        voxels[idx] = voxel;
                        idx += 1;
                    }
                }
            }
        }
    }

    #[test]
    fn should_yield_empty_object_when_generating_object_with_empty_grid() {
        assert!(
            ChunkedVoxelObject::generate_without_derived_state(
                &OffsetBoxVoxelGenerator::with_default([0; 3])
            )
            .contains_only_empty_voxels()
        );
    }

    #[test]
    fn should_yield_empty_object_when_generating_object_of_empty_voxels() {
        assert!(
            ChunkedVoxelObject::generate_without_derived_state(
                &OffsetBoxVoxelGenerator::single_empty()
            )
            .contains_only_empty_voxels()
        );
        assert!(
            ChunkedVoxelObject::generate_without_derived_state(&OffsetBoxVoxelGenerator::empty([
                2, 3, 4
            ]))
            .contains_only_empty_voxels()
        );
    }

    #[test]
    fn should_generate_object_with_single_voxel() {
        let generator = OffsetBoxVoxelGenerator::single_default();
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        assert_eq!(object.voxel_extent(), generator.voxel_extent());
        assert_eq!(object.chunk_counts(), &[1, 1, 1]);
        assert_eq!(object.occupied_voxel_ranges()[0], 0..CHUNK_SIZE);
        assert_eq!(object.occupied_voxel_ranges()[1], 0..CHUNK_SIZE);
        assert_eq!(object.occupied_voxel_ranges()[2], 0..CHUNK_SIZE);
        assert_eq!(object.stored_voxel_count(), CHUNK_VOXEL_COUNT);
    }

    #[test]
    fn should_generate_object_with_single_uniform_chunk() {
        let generator = OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        assert_eq!(object.chunk_counts(), &[1, 1, 1]);
        assert_eq!(object.occupied_voxel_ranges()[0], 0..CHUNK_SIZE);
        assert_eq!(object.occupied_voxel_ranges()[1], 0..CHUNK_SIZE);
        assert_eq!(object.occupied_voxel_ranges()[2], 0..CHUNK_SIZE);
        assert_eq!(object.stored_voxel_count(), 1);
    }

    #[test]
    fn should_generate_object_with_single_offset_uniform_chunk() {
        let generator =
            OffsetBoxVoxelGenerator::offset_with_default([CHUNK_SIZE; 3], [CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        assert_eq!(object.chunk_counts(), &[2, 2, 2]);
        assert_eq!(
            object.occupied_voxel_ranges()[0],
            CHUNK_SIZE..2 * CHUNK_SIZE
        );
        assert_eq!(
            object.occupied_voxel_ranges()[1],
            CHUNK_SIZE..2 * CHUNK_SIZE
        );
        assert_eq!(
            object.occupied_voxel_ranges()[2],
            CHUNK_SIZE..2 * CHUNK_SIZE
        );
        assert_eq!(object.stored_voxel_count(), 1);
    }

    #[test]
    fn should_get_correct_voxels_in_small_grid() {
        let generator = ManualVoxelGenerator::<3>::new([
            [[1, 1, 0], [1, 0, 1], [0, 1, 0]],
            [[0, 1, 1], [1, 0, 0], [1, 0, 1]],
            [[1, 1, 0], [1, 1, 1], [0, 0, 0]],
        ]);
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    assert_eq!(
                        object.get_voxel(i, j, k).map_or(0, |_| 1),
                        generator.voxels[i][j][k]
                    );
                }
            }
        }
    }

    #[test]
    fn should_get_correct_voxels_in_small_offset_grid() {
        let offset = [CHUNK_SIZE - 2; 3];
        let generator = ManualVoxelGenerator::<3>::with_offset(
            [
                [[1, 1, 0], [1, 0, 1], [0, 1, 0]],
                [[0, 1, 1], [1, 0, 0], [1, 0, 1]],
                [[1, 1, 0], [1, 1, 1], [0, 0, 0]],
            ],
            offset,
        );
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    assert_eq!(
                        object
                            .get_voxel(offset[0] + i, offset[1] + j, offset[2] + k)
                            .map_or(0, |_| 1),
                        generator.voxels[i][j][k]
                    );
                }
            }
        }
    }

    #[test]
    fn should_compute_correct_internal_adjacency_in_chunk() {
        let generator = ManualVoxelGenerator::<3>::new([
            [[0, 0, 0], [0, 1, 0], [0, 0, 0]],
            [[0, 1, 0], [1, 1, 1], [0, 1, 0]],
            [[0, 0, 0], [0, 1, 0], [0, 0, 0]],
        ]);

        let object = ChunkedVoxelObject::generate(&generator);

        assert_eq!(
            object.get_voxel(1, 1, 1).unwrap().flags(),
            VoxelFlags::full_adjacency()
        );
        assert_eq!(
            object.get_voxel(0, 1, 1).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_X_UP
        );
        assert_eq!(
            object.get_voxel(2, 1, 1).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_X_DN
        );
        assert_eq!(
            object.get_voxel(1, 0, 1).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_Y_UP
        );
        assert_eq!(
            object.get_voxel(1, 2, 1).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_Y_DN
        );
        assert_eq!(
            object.get_voxel(1, 1, 0).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_Z_UP
        );
        assert_eq!(
            object.get_voxel(1, 1, 2).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_Z_DN
        );
    }

    #[test]
    fn should_compute_correct_internal_adjacency_in_lower_chunk_corner() {
        let generator = ManualVoxelGenerator::<3>::new([
            [[1, 1, 0], [1, 0, 0], [0, 0, 0]],
            [[1, 0, 0], [0, 0, 0], [0, 0, 0]],
            [[0, 0, 0], [0, 0, 0], [0, 0, 0]],
        ]);

        let object = ChunkedVoxelObject::generate(&generator);

        assert_eq!(
            object.get_voxel(0, 0, 0).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_X_UP
                | VoxelFlags::HAS_ADJACENT_Y_UP
                | VoxelFlags::HAS_ADJACENT_Z_UP
        );
        assert_eq!(
            object.get_voxel(0, 0, 1).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_Z_DN
        );
        assert_eq!(
            object.get_voxel(0, 1, 0).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_Y_DN
        );
        assert_eq!(
            object.get_voxel(1, 0, 0).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_X_DN
        );
    }

    #[test]
    fn should_compute_correct_internal_adjacency_in_upper_chunk_corner() {
        let offset = [CHUNK_SIZE - 3; 3];
        let generator = ManualVoxelGenerator::<3>::with_offset(
            [
                [[0, 0, 0], [0, 0, 0], [0, 0, 0]],
                [[0, 0, 0], [0, 0, 0], [0, 0, 1]],
                [[0, 0, 0], [0, 0, 1], [0, 1, 1]],
            ],
            offset,
        );

        let object = ChunkedVoxelObject::generate(&generator);

        assert_eq!(
            object
                .get_voxel(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 1)
                .unwrap()
                .flags(),
            VoxelFlags::HAS_ADJACENT_X_DN
                | VoxelFlags::HAS_ADJACENT_Y_DN
                | VoxelFlags::HAS_ADJACENT_Z_DN
        );
        assert_eq!(
            object
                .get_voxel(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 2)
                .unwrap()
                .flags(),
            VoxelFlags::HAS_ADJACENT_Z_UP
        );
        assert_eq!(
            object
                .get_voxel(CHUNK_SIZE - 1, CHUNK_SIZE - 2, CHUNK_SIZE - 1)
                .unwrap()
                .flags(),
            VoxelFlags::HAS_ADJACENT_Y_UP
        );
        assert_eq!(
            object
                .get_voxel(CHUNK_SIZE - 2, CHUNK_SIZE - 1, CHUNK_SIZE - 1)
                .unwrap()
                .flags(),
            VoxelFlags::HAS_ADJACENT_X_UP
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn should_compute_correct_adjacencies_for_single_voxel() {
        let generator = OffsetBoxVoxelGenerator::with_default([1; 3]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
    }

    #[test]
    #[cfg(not(miri))]
    fn should_compute_correct_adjacencies_for_single_chunk() {
        let generator = OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
    }

    #[test]
    #[cfg(not(miri))]
    fn should_compute_correct_adjacencies_for_barely_two_chunks() {
        let generator =
            OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE + 1, CHUNK_SIZE, CHUNK_SIZE]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator =
            OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE, CHUNK_SIZE + 1, CHUNK_SIZE]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator =
            OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE + 1]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
    }

    #[test]
    #[cfg(not(miri))]
    fn should_compute_correct_adjacencies_with_column_taking_barely_two_chunks() {
        let generator = OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE + 1, 1, 1]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator = OffsetBoxVoxelGenerator::with_default([1, CHUNK_SIZE + 1, 1]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator = OffsetBoxVoxelGenerator::with_default([1, 1, CHUNK_SIZE + 1]);
        let object = ChunkedVoxelObject::generate(&generator);
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
    }

    #[test]
    fn should_mark_correct_lower_face_as_obscured_for_chunk_flags() {
        let mut flags = VoxelChunkFlags::empty();
        flags.mark_lower_face_as_obscured(Dimension::X);
        assert_eq!(flags, VoxelChunkFlags::IS_OBSCURED_X_DN);

        let mut flags = VoxelChunkFlags::empty();
        flags.mark_lower_face_as_obscured(Dimension::Y);
        assert_eq!(flags, VoxelChunkFlags::IS_OBSCURED_Y_DN);

        let mut flags = VoxelChunkFlags::empty();
        flags.mark_lower_face_as_obscured(Dimension::Z);
        assert_eq!(flags, VoxelChunkFlags::IS_OBSCURED_Z_DN);
    }

    #[test]
    fn should_mark_correct_upper_face_as_obscured_for_chunk_flags() {
        let mut flags = VoxelChunkFlags::empty();
        flags.mark_upper_face_as_obscured(Dimension::X);
        assert_eq!(flags, VoxelChunkFlags::IS_OBSCURED_X_UP);

        let mut flags = VoxelChunkFlags::empty();
        flags.mark_upper_face_as_obscured(Dimension::Y);
        assert_eq!(flags, VoxelChunkFlags::IS_OBSCURED_Y_UP);

        let mut flags = VoxelChunkFlags::empty();
        flags.mark_upper_face_as_obscured(Dimension::Z);
        assert_eq!(flags, VoxelChunkFlags::IS_OBSCURED_Z_UP);
    }

    #[test]
    fn should_mark_correct_lower_face_as_unobscured_for_chunk_flags() {
        let mut flags = VoxelChunkFlags::all();
        flags.mark_lower_face_as_unobscured(Dimension::X);
        assert_eq!(
            flags,
            VoxelChunkFlags::all() - VoxelChunkFlags::IS_OBSCURED_X_DN
        );

        let mut flags = VoxelChunkFlags::all();
        flags.mark_lower_face_as_unobscured(Dimension::Y);
        assert_eq!(
            flags,
            VoxelChunkFlags::all() - VoxelChunkFlags::IS_OBSCURED_Y_DN
        );

        let mut flags = VoxelChunkFlags::all();
        flags.mark_lower_face_as_unobscured(Dimension::Z);
        assert_eq!(
            flags,
            VoxelChunkFlags::all() - VoxelChunkFlags::IS_OBSCURED_Z_DN
        );
    }

    #[test]
    fn should_mark_correct_upper_face_as_unobscured_for_chunk_flags() {
        let mut flags = VoxelChunkFlags::all();
        flags.mark_upper_face_as_unobscured(Dimension::X);
        assert_eq!(
            flags,
            VoxelChunkFlags::all() - VoxelChunkFlags::IS_OBSCURED_X_UP
        );

        let mut flags = VoxelChunkFlags::all();
        flags.mark_upper_face_as_unobscured(Dimension::Y);
        assert_eq!(
            flags,
            VoxelChunkFlags::all() - VoxelChunkFlags::IS_OBSCURED_Y_UP
        );

        let mut flags = VoxelChunkFlags::all();
        flags.mark_upper_face_as_unobscured(Dimension::Z);
        assert_eq!(
            flags,
            VoxelChunkFlags::all() - VoxelChunkFlags::IS_OBSCURED_Z_UP
        );
    }

    #[test]
    fn should_compute_correct_aabb_for_single_voxel() {
        let generator = OffsetBoxVoxelGenerator::with_default([1; 3]);
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(aabb.lower_corner(), &Point3::new(0.0, 0.0, 0.0));
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            // The occupied voxel range has chunk granularity, so the AABB will never be smaller
            // than a single chunk
            &Point3::new(
                generator.voxel_extent() * CHUNK_SIZE as f32,
                generator.voxel_extent() * CHUNK_SIZE as f32,
                generator.voxel_extent() * CHUNK_SIZE as f32,
            )
        );
    }

    #[test]
    fn should_compute_correct_aabb_for_single_chunk() {
        let generator = OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(aabb.lower_corner(), &Point3::new(0.0, 0.0, 0.0));
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &Point3::new(
                generator.voxel_extent() * CHUNK_SIZE as f32,
                generator.voxel_extent() * CHUNK_SIZE as f32,
                generator.voxel_extent() * CHUNK_SIZE as f32,
            )
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn should_compute_correct_aabb_for_different_numbers_of_chunks_along_each_axis() {
        let generator =
            OffsetBoxVoxelGenerator::with_default([2 * CHUNK_SIZE, 3 * CHUNK_SIZE, 4 * CHUNK_SIZE]);
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(aabb.lower_corner(), &Point3::new(0.0, 0.0, 0.0));
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &Point3::new(
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f32,
                generator.voxel_extent() * (3 * CHUNK_SIZE) as f32,
                generator.voxel_extent() * (4 * CHUNK_SIZE) as f32,
            )
        );
    }

    #[test]
    fn should_shrink_occupied_voxel_ranges_correctly_for_single_voxel() {
        let generator = OffsetBoxVoxelGenerator::single_default();
        let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        object.update_occupied_voxel_ranges();

        assert_eq!(object.occupied_voxel_ranges, [0..1, 0..1, 0..1]);
    }

    #[test]
    fn should_shrink_occupied_voxel_ranges_correctly_for_multiple_chunks() {
        let generator = OffsetBoxVoxelGenerator::new(
            [9, 9, 9],
            [0, 0, 0],
            Voxel::maximally_inside(VoxelType::default()),
        );
        let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        object.update_occupied_voxel_ranges();

        assert_eq!(object.occupied_voxel_ranges, [0..9, 0..9, 0..9]);
    }

    #[test]
    fn should_shrink_occupied_voxel_ranges_correctly_for_offset_chunk() {
        let generator = OffsetBoxVoxelGenerator::offset_with_default([1, 1, 1], [5, 5, 5]);
        let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        object.update_occupied_voxel_ranges();

        assert_eq!(object.occupied_voxel_ranges, [5..6, 5..6, 5..6]);
    }

    #[test]
    fn should_shrink_occupied_voxel_ranges_correctly_for_sparse_multi_chunk() {
        const GRID_SIZE: usize = 20;
        let mut voxels = [[[0u8; GRID_SIZE]; GRID_SIZE]; GRID_SIZE];

        voxels[2][2][5] = 1; // Min boundaries (chunk 0)
        voxels[18][17][19] = 1; // Max boundaries (chunk 1)
        voxels[3][3][6] = 1; // Additional voxel near minimums
        voxels[17][16][18] = 1; // Additional voxel near maximums

        let generator = ManualVoxelGenerator::new(voxels);
        let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        object.update_occupied_voxel_ranges();

        assert_eq!(object.occupied_voxel_ranges, [2..19, 2..18, 5..20]);
    }

    #[test]
    fn should_compute_correct_aabb_for_offset_chunk() {
        let generator =
            OffsetBoxVoxelGenerator::offset_with_default([CHUNK_SIZE; 3], [CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(
            aabb.lower_corner(),
            &Point3::new(
                generator.voxel_extent() * CHUNK_SIZE as f32,
                generator.voxel_extent() * CHUNK_SIZE as f32,
                generator.voxel_extent() * CHUNK_SIZE as f32,
            )
        );
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &Point3::new(
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f32,
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f32,
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f32,
            )
        );
    }
}
