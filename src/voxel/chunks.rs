//! Chunked representation of voxel objects.

pub mod intersection;
pub mod sdf;

use crate::{
    geometry::{AxisAlignedBox, Sphere},
    num::Float,
    voxel::{
        generation::VoxelGenerator,
        utils::{DataLoop3, Dimension, Loop3, MutDataLoop3, Side},
        voxel_types::{VoxelType, VoxelTypeRegistry},
        Voxel, VoxelFlags,
    },
};
use bitflags::bitflags;
use nalgebra::point;
use num_traits::{NumCast, PrimInt};
use std::{collections::HashSet, iter, ops::Range};

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
    voxel_extent: f64,
    chunk_counts: [usize; 3],
    chunk_idx_strides: [usize; 2],
    occupied_chunk_ranges: [Range<usize>; 3],
    occupied_voxel_ranges: [Range<usize>; 3],
    chunks: Vec<VoxelChunk>,
    voxels: Vec<Voxel>,
    _voxel_types: Vec<VoxelType>,
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
    Uniform(Voxel),
    NonUniform(NonUniformVoxelChunk),
}

/// A non-uniform chunk representing a cubic grid of voxel chunks. The chunk is
/// not fully packed and/or contains a mix of voxels with different information.
/// The voxels comprising the non-uniform chunk are stored in the parent
/// [`ChunkedVoxelObject`], and the chunk stores an offset to its voxel data as
/// well as information on the distribution of voxels across the faces of the
/// chunk and a set of flags encoding additional information about the state of
/// the chunk.
#[derive(Clone, Copy, Debug)]
pub struct NonUniformVoxelChunk {
    data_offset: u32,
    face_distributions: [[FaceVoxelDistribution; 2]; 3],
    flags: VoxelChunkFlags,
}

/// Information about the distribution of voxels across a specific face of a
/// chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum FaceVoxelDistribution {
    /// There are no voxels on the face.
    Empty,
    /// The face is completely filled with voxels (but they may have different
    /// properties).
    Full,
    /// The face is partially filled with voxels.
    Mixed,
}

bitflags! {
    /// Bitflags encoding a set of potential binary states for a voxel chunk.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Generates a new `ChunkedVoxelObject` using the given [`VoxelGenerator`]
    /// and calls [`Self::initialize_adjacencies`] on it. Returns [`None`]
    /// if the resulting object would not contain any voxels.
    pub fn generate<G>(generator: &G) -> Option<Self>
    where
        G: VoxelGenerator,
    {
        let mut object = Self::generate_without_adjacencies(generator)?;
        object.initialize_adjacencies();
        Some(object)
    }

    /// Generates a new `ChunkedVoxelObject` using the given [`VoxelGenerator`].
    /// Returns [`None`] if the resulting object would not contain any voxels.
    pub fn generate_without_adjacencies<G>(generator: &G) -> Option<Self>
    where
        G: VoxelGenerator,
    {
        let generator_grid_shape = generator.grid_shape();

        if generator_grid_shape.iter().any(|&dim| dim == 0) {
            return None;
        }

        let chunk_counts = generator_grid_shape.map(|size| size.div_ceil(CHUNK_SIZE));
        let chunk_idx_strides = [chunk_counts[1] * chunk_counts[2], chunk_counts[2]];

        let mut chunks = Vec::with_capacity(chunk_counts.iter().product());
        let mut voxels = Vec::new();

        let mut occupied_chunks_i = REVERSED_MAX_RANGE;
        let mut occupied_chunks_j = REVERSED_MAX_RANGE;
        let mut occupied_chunks_k = REVERSED_MAX_RANGE;

        for chunk_i in 0..chunk_counts[0] {
            for chunk_j in 0..chunk_counts[1] {
                for chunk_k in 0..chunk_counts[2] {
                    let chunk =
                        VoxelChunk::generate(&mut voxels, generator, [chunk_i, chunk_j, chunk_k]);

                    if !chunk.is_empty() {
                        occupied_chunks_i.start = occupied_chunks_i.start.min(chunk_i);
                        occupied_chunks_i.end = occupied_chunks_i.end.max(chunk_i + 1);
                        occupied_chunks_j.start = occupied_chunks_j.start.min(chunk_j);
                        occupied_chunks_j.end = occupied_chunks_j.end.max(chunk_j + 1);
                        occupied_chunks_k.start = occupied_chunks_k.start.min(chunk_k);
                        occupied_chunks_k.end = occupied_chunks_k.end.max(chunk_k + 1);
                    }

                    chunks.push(chunk);
                }
            }
        }

        let occupied_chunk_ranges = [occupied_chunks_i, occupied_chunks_j, occupied_chunks_k];

        if occupied_chunk_ranges.iter().any(Range::is_empty) {
            return None;
        }

        let occupied_voxel_ranges = occupied_chunk_ranges
            .clone()
            .map(|chunk_range| chunk_range.start * CHUNK_SIZE..chunk_range.end * CHUNK_SIZE);

        let voxel_types = Self::find_voxel_types(&chunks, &voxels);

        Some(Self {
            voxel_extent: generator.voxel_extent(),
            chunk_counts,
            chunk_idx_strides,
            occupied_chunk_ranges,
            occupied_voxel_ranges,
            chunks,
            voxels,
            _voxel_types: voxel_types,
            invalidated_mesh_chunk_indices: HashSet::new(),
        })
    }

    fn find_voxel_types(chunks: &[VoxelChunk], voxels: &[Voxel]) -> Vec<VoxelType> {
        let mut has_voxel_type = [false; VoxelTypeRegistry::max_n_voxel_types() + 1];

        for chunk in chunks {
            if let VoxelChunk::Uniform(voxel) = chunk {
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
    pub fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    /// Returns the extent of single voxel chunk in the object.
    pub fn chunk_extent(&self) -> f64 {
        self.voxel_extent * CHUNK_SIZE as f64
    }

    /// Returns the number of chunks along each axis of the object's voxel
    /// grid.
    pub fn chunk_counts(&self) -> &[usize; 3] {
        &self.chunk_counts
    }

    /// Returns the total number of chunks, incuding empty ones, contained in
    /// the object's chunk grid.
    pub fn total_chunk_count(&self) -> usize {
        self.chunk_counts.iter().product()
    }

    /// Returns a guess for the rough number of exposed chunks the object
    /// contains based on its size.
    pub fn exposed_chunk_count_heuristic(&self) -> usize {
        // It is probably roughly equal to the total number of boundary chunks
        2 * (self.chunk_counts[0] * self.chunk_counts[1]
            + self.chunk_counts[1] * self.chunk_counts[2]
            + self.chunk_counts[2] * self.chunk_counts[0])
    }

    /// Returns the range of indices along each axis of the object's chunk
    /// grid that may contain non-empty chunks.
    pub fn occupied_chunk_ranges(&self) -> &[Range<usize>] {
        &self.occupied_chunk_ranges
    }

    /// Returns the range of indices along each axis of the object's voxel
    /// grid that may contain non-empty voxels.
    pub fn occupied_voxel_ranges(&self) -> &[Range<usize>] {
        &self.occupied_voxel_ranges
    }

    /// Returns the number of voxels (potentially empty) actually stored in the
    /// object (as opposed to the count of voxels the object logically
    /// contains).
    pub fn stored_voxel_count(&self) -> usize {
        self.chunks
            .iter()
            .map(|chunk| chunk.stored_voxel_count())
            .sum()
    }

    /// Computes the axis-aligned bounding box enclosing all non-empty voxels in
    /// the object.
    pub fn compute_aabb<F: Float>(&self) -> AxisAlignedBox<F> {
        let voxel_extent = F::from_f64(self.voxel_extent()).unwrap();

        let lower_corner = point![
            F::from_usize(self.occupied_voxel_ranges[0].start).unwrap() * voxel_extent,
            F::from_usize(self.occupied_voxel_ranges[1].start).unwrap() * voxel_extent,
            F::from_usize(self.occupied_voxel_ranges[2].start).unwrap() * voxel_extent
        ];

        let upper_corner = point![
            F::from_usize(self.occupied_voxel_ranges[0].end).unwrap() * voxel_extent,
            F::from_usize(self.occupied_voxel_ranges[1].end).unwrap() * voxel_extent,
            F::from_usize(self.occupied_voxel_ranges[2].end).unwrap() * voxel_extent
        ];

        AxisAlignedBox::new(lower_corner, upper_corner)
    }

    /// Computes a sphere enclosing all non-empty voxels in the object.
    pub fn compute_bounding_sphere<F: Float>(&self) -> Sphere<F> {
        Sphere::bounding_sphere_from_aabb(&self.compute_aabb())
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
    pub fn non_uniform_chunk_voxels(&self, chunk: &NonUniformVoxelChunk) -> &[Voxel] {
        let start_voxel_idx = chunk_start_voxel_idx(chunk.data_offset);
        &self.voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT]
    }

    /// Returns a reference to the voxel at the given indices in the object's
    /// voxel grid, or [`None`] if the voxel is empty or the indices are out of
    /// bounds.
    ///
    /// Despite the organization of voxels into chunks, this lookup is
    /// relatively efficient because we can perform simple bit manipulations
    /// to determine the chunk containing the voxel.
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

        let chunk_idx = self.linear_chunk_idx_from_object_voxel_indices(i, j, k);
        let chunk = &self.chunks[chunk_idx];
        match &chunk {
            VoxelChunk::Empty => None,
            VoxelChunk::Uniform(voxel) => Some(voxel),
            VoxelChunk::NonUniform(NonUniformVoxelChunk { data_offset, .. }) => {
                let voxel_idx = chunk_start_voxel_idx(*data_offset)
                    + linear_voxel_idx_within_chunk_from_object_voxel_indices(i, j, k);
                let voxel = &self.voxels[voxel_idx];
                if voxel.is_empty() {
                    None
                } else {
                    Some(voxel)
                }
            }
        }
    }

    /// Returns the [`VoxelChunk`] at the given indices in the object's chunk
    /// grid. If the indices are out of bounds, an empty chunk is returned.
    pub fn get_chunk<I: PrimInt>(&self, chunk_i: I, chunk_j: I, chunk_k: I) -> VoxelChunk {
        if chunk_i < I::from(self.occupied_chunk_ranges[0].start).unwrap()
            || chunk_j < I::from(self.occupied_chunk_ranges[1].start).unwrap()
            || chunk_k < I::from(self.occupied_chunk_ranges[2].start).unwrap()
            || chunk_i >= I::from(self.occupied_chunk_ranges[0].end).unwrap()
            || chunk_j >= I::from(self.occupied_chunk_ranges[1].end).unwrap()
            || chunk_k >= I::from(self.occupied_chunk_ranges[2].end).unwrap()
        {
            return VoxelChunk::Empty;
        }

        let chunk_i = NumCast::from(chunk_i).unwrap();
        let chunk_j = NumCast::from(chunk_j).unwrap();
        let chunk_k = NumCast::from(chunk_k).unwrap();

        let chunk_idx = self.linear_chunk_idx(&[chunk_i, chunk_j, chunk_k]);
        self.chunks[chunk_idx]
    }

    /// Determines the adjacency [`VoxelFlags`] for each voxel in the object
    /// according to which of their six neighbor voxels are present. Also
    /// records which faces of the chunks are fully obscured by adjacent voxels.
    pub fn initialize_adjacencies(&mut self) {
        for chunk in &self.chunks {
            chunk.update_internal_adjacencies(self.voxels.as_mut_slice());
        }
        self.update_all_chunk_boundary_adjacencies();
    }

    /// Returns an iterator over the indices in the object's chunk grid of the
    /// chunks whose (hypothetical) meshes have been invalidated by changes in
    /// the voxel object since the object was created or
    /// [`Self::mark_chunk_meshes_synchronized`] was last called.
    pub fn invalidated_mesh_chunk_indices(
        &self,
    ) -> impl ExactSizeIterator<Item = &[usize; 3]> + '_ {
        self.invalidated_mesh_chunk_indices.iter()
    }

    /// Signals that the mesh data of all the object's chunks is up to date with
    /// the object's voxels.
    pub fn mark_chunk_meshes_synchronized(&mut self) {
        self.invalidated_mesh_chunk_indices.clear();
    }

    /// Validates the adjacency [`VoxelFlags`] computed by the efficient
    /// [`Self::initialize_adjacencies`] method by performing a simple
    /// brute-force iteration over all voxels and checking their neighbors.
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
                if let Some(voxel) = self.get_voxel(0, j, k) {
                    if voxel.flags().contains(VoxelFlags::HAS_ADJACENT_X_DN) {
                        invalid_present_flags.push(([0, j, k], VoxelFlags::HAS_ADJACENT_X_DN));
                    }
                }
            }
        }
        for i in self.occupied_voxel_ranges[0].clone() {
            for k in self.occupied_voxel_ranges[2].clone() {
                if let Some(voxel) = self.get_voxel(i, 0, k) {
                    if voxel.flags().contains(VoxelFlags::HAS_ADJACENT_Y_DN) {
                        invalid_present_flags.push(([i, 0, k], VoxelFlags::HAS_ADJACENT_Y_DN));
                    }
                }
            }
        }
        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                if let Some(voxel) = self.get_voxel(i, j, 0) {
                    if voxel.flags().contains(VoxelFlags::HAS_ADJACENT_Z_DN) {
                        invalid_present_flags.push(([i, j, 0], VoxelFlags::HAS_ADJACENT_Z_DN));
                    }
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
    /// [`Self::initialize_adjacencies`] method for chunks by performing a
    /// simple brute-force iteration over all chunks and checking their
    /// neighbors.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_chunk_obscuredness(&self) {
        use super::utils::Dimension;

        let mut invalid_missing_flags = Vec::new();
        let mut invalid_present_flags = Vec::new();
        let mut invalid_uniform = Vec::new();

        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_j in self.occupied_chunk_ranges[1].clone() {
                for chunk_k in self.occupied_chunk_ranges[2].clone() {
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

        for chunk_j in self.occupied_chunk_ranges[1].clone() {
            for chunk_k in self.occupied_chunk_ranges[2].clone() {
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
        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_k in self.occupied_chunk_ranges[2].clone() {
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
        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_j in self.occupied_chunk_ranges[1].clone() {
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

    fn update_all_chunk_boundary_adjacencies(&mut self) {
        self.update_upper_boundary_adjacencies_for_chunks_in_ranges(
            self.occupied_chunk_ranges.clone(),
        );

        // Handle lower faces of the full object, since these are not included
        // in the loop above
        for chunk_j in self.occupied_chunk_ranges[1].clone() {
            for chunk_k in self.occupied_chunk_ranges[2].clone() {
                let chunk_idx = self.linear_chunk_idx(&[0, chunk_j, chunk_k]);
                VoxelChunk::update_mutual_face_adjacencies(
                    &mut self.chunks,
                    &mut self.voxels,
                    None,
                    Some(chunk_idx),
                    Dimension::X,
                );
            }
        }
        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_k in self.occupied_chunk_ranges[2].clone() {
                let chunk_idx = self.linear_chunk_idx(&[chunk_i, 0, chunk_k]);
                VoxelChunk::update_mutual_face_adjacencies(
                    &mut self.chunks,
                    &mut self.voxels,
                    None,
                    Some(chunk_idx),
                    Dimension::Y,
                );
            }
        }
        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_j in self.occupied_chunk_ranges[1].clone() {
                let chunk_idx = self.linear_chunk_idx(&[chunk_i, chunk_j, 0]);
                VoxelChunk::update_mutual_face_adjacencies(
                    &mut self.chunks,
                    &mut self.voxels,
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
    fn linear_chunk_idx_from_object_voxel_indices(&self, i: usize, j: usize, k: usize) -> usize {
        let chunk_indices = chunk_indices_from_object_voxel_indices(i, j, k);
        self.linear_chunk_idx(&chunk_indices)
    }

    /// Computes the index in `self.chunks` of the chunk with the given 3D index
    /// in the object's chunk grid.
    fn linear_chunk_idx(&self, chunk_indices: &[usize; 3]) -> usize {
        chunk_indices[0] * self.chunk_idx_strides[0]
            + chunk_indices[1] * self.chunk_idx_strides[1]
            + chunk_indices[2]
    }
}

impl VoxelChunk {
    fn generate<G>(voxels: &mut Vec<Voxel>, generator: &G, chunk_indices: [usize; 3]) -> Self
    where
        G: VoxelGenerator,
    {
        let origin = [
            chunk_indices[0] * CHUNK_SIZE,
            chunk_indices[1] * CHUNK_SIZE,
            chunk_indices[2] * CHUNK_SIZE,
        ];

        let mut first_voxel = generator.voxel_at_indices(origin[0], origin[1], origin[2]);
        let mut is_uniform = true;

        let start_voxel_idx = voxels.len();
        voxels.reserve(CHUNK_VOXEL_COUNT);

        let mut face_empty_counts = FaceEmptyCounts::zero();

        let range_i = origin[0]..origin[0] + CHUNK_SIZE;
        let range_j = origin[1]..origin[1] + CHUNK_SIZE;
        let range_k = origin[2]..origin[2] + CHUNK_SIZE;

        for i in range_i.clone() {
            for j in range_j.clone() {
                for k in range_k.clone() {
                    let voxel = generator.voxel_at_indices(i, j, k);

                    if is_uniform && voxel != first_voxel {
                        is_uniform = false;
                    }

                    if voxel.is_empty() {
                        if i == range_i.start {
                            face_empty_counts.increment_x_dn();
                        } else if i == range_i.end - 1 {
                            face_empty_counts.increment_x_up();
                        }
                        if j == range_j.start {
                            face_empty_counts.increment_y_dn();
                        } else if j == range_j.end - 1 {
                            face_empty_counts.increment_y_up();
                        }
                        if k == range_k.start {
                            face_empty_counts.increment_z_dn();
                        } else if k == range_k.end - 1 {
                            face_empty_counts.increment_z_up();
                        }
                    }

                    voxels.push(voxel);
                }
            }
        }

        if is_uniform {
            voxels.truncate(start_voxel_idx);

            if first_voxel.is_empty() {
                Self::Empty
            } else {
                // If the chunk has truly uniform information, even the boundary voxels must be
                // fully surrounded by neighbors. We don't know if this is the case yet, but we
                // assume it to be true and fix it by making the chunk non-uniform later if it
                // turns out not to be the case
                first_voxel.add_flags(VoxelFlags::full_adjacency());

                Self::Uniform(first_voxel)
            }
        } else {
            let face_distributions = face_empty_counts.to_face_distributions(CHUNK_SIZE_SQUARED);

            Self::NonUniform(NonUniformVoxelChunk {
                data_offset: chunk_data_offset_from_start_voxel_idx(start_voxel_idx),
                face_distributions,
                flags: VoxelChunkFlags::empty(),
            })
        }
    }

    const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    const fn data_offset_if_non_uniform(&self) -> Option<u32> {
        if let Self::NonUniform(NonUniformVoxelChunk { data_offset, .. }) = self {
            Some(*data_offset)
        } else {
            None
        }
    }

    const fn start_voxel_idx_if_non_uniform(&self) -> Option<usize> {
        if let Some(data_offset) = self.data_offset_if_non_uniform() {
            Some(chunk_start_voxel_idx(data_offset))
        } else {
            None
        }
    }

    const fn stored_voxel_count(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Uniform(_) => 1,
            Self::NonUniform(_) => CHUNK_VOXEL_COUNT,
        }
    }

    fn upper_face_voxel_distribution(&self, dim: Dimension) -> FaceVoxelDistribution {
        match self {
            Self::Empty => FaceVoxelDistribution::Empty,
            Self::Uniform(_) => FaceVoxelDistribution::Full,
            Self::NonUniform(NonUniformVoxelChunk {
                face_distributions, ..
            }) => face_distributions[dim.idx()][1],
        }
    }

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
        let start_voxel_idx = if let Some(start_voxel_idx) = self.start_voxel_idx_if_non_uniform() {
            start_voxel_idx
        } else {
            return;
        };

        // Extract the sub-slice of voxels for this chunk so that we get
        // out-of-bounds if trying to access voxels outside the chunk
        let chunk_voxels = &mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT];

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let idx = linear_voxel_idx_within_chunk(&[i, j, k]);

                    if chunk_voxels[idx].is_empty() {
                        continue;
                    }

                    let mut flags = VoxelFlags::empty();

                    // Since we will update the flag of the adjacent voxel in
                    // addition to this one, we only need to look up the upper
                    // adjacent voxels to cover every adjacency over the course
                    // of the full loop
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
                            let adjacent_voxel = &mut chunk_voxels[adjacent_idx];
                            if !adjacent_voxel.is_empty() {
                                flags |= flag_for_current;
                                adjacent_voxel.add_flags(flag_for_adjacent);
                            }
                        }
                    }

                    chunk_voxels[idx].add_flags(flags);
                }
            }
        }
    }

    fn update_face_distributions_and_internal_adjacencies(&mut self, voxels: &mut [Voxel]) {
        // We only need to update the face distributions and internal adjacencies if the
        // chunk is non-uniform
        let (start_voxel_idx, face_distributions) =
            if let Self::NonUniform(NonUniformVoxelChunk {
                data_offset,
                face_distributions,
                ..
            }) = self
            {
                (chunk_start_voxel_idx(*data_offset), face_distributions)
            } else {
                return;
            };

        // Extract the sub-slice of voxels for this chunk so that we get
        // out-of-bounds if trying to access voxels outside the chunk
        let chunk_voxels = &mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT];

        let mut face_empty_counts = FaceEmptyCounts::zero();

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let idx = linear_voxel_idx_within_chunk(&[i, j, k]);

                    if chunk_voxels[idx].is_empty() {
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
                        continue;
                    }

                    let mut flags = VoxelFlags::empty();

                    // Since we will update the flag of the adjacent voxel in
                    // addition to this one, we only need to look up the upper
                    // adjacent voxels to cover every adjacency over the course
                    // of the full loop
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
                            let adjacent_voxel = &mut chunk_voxels[adjacent_idx];
                            if !adjacent_voxel.is_empty() {
                                flags |= flag_for_current;
                                adjacent_voxel.add_flags(flag_for_adjacent);
                            }
                        }
                    }

                    chunk_voxels[idx].add_flags(flags);
                }
            }
        }

        *face_distributions = face_empty_counts.to_face_distributions(CHUNK_SIZE_SQUARED);
    }

    fn mark_lower_face_as_obscured(&mut self, dim: Dimension) {
        let flags = match self {
            Self::Empty | Self::Uniform(_) => {
                return;
            }
            Self::NonUniform(NonUniformVoxelChunk { flags, .. }) => flags,
        };
        flags.mark_lower_face_as_obscured(dim);
    }

    fn mark_upper_face_as_obscured(&mut self, dim: Dimension) {
        let flags = match self {
            Self::Empty | Self::Uniform(_) => {
                return;
            }
            Self::NonUniform(NonUniformVoxelChunk { flags, .. }) => flags,
        };
        flags.mark_upper_face_as_obscured(dim);
    }

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
        lower_chunk_idx: Option<usize>,
        upper_chunk_idx: Option<usize>,
        dim: Dimension,
    ) {
        let lower_chunk =
            lower_chunk_idx.map_or_else(|| VoxelChunk::Empty, |chunk_idx| chunks[chunk_idx]);
        let upper_chunk =
            upper_chunk_idx.map_or_else(|| VoxelChunk::Empty, |chunk_idx| chunks[chunk_idx]);

        match (lower_chunk, upper_chunk) {
            // If both chunks are empty or uniform, there is nothing to do
            // (uniform chunks are always marked as fully obscured upon
            // creation, so we don't have to update their obscuredness)
            (Self::Empty, Self::Empty) | (Self::Uniform(_), Self::Uniform(_)) => {}
            // If one is uniform and the other is empty, we need to convert the
            // uniform chunk to non-uniform and clear its adjacencies to the
            // empty chunk, as well as mark the adjoining face of the uniform
            // chunk as unobscured
            (Self::Uniform(_), Self::Empty) => {
                let lower_chunk = &mut chunks[lower_chunk_idx.unwrap()];
                lower_chunk.convert_to_non_uniform_if_uniform(voxels);

                Self::remove_all_outward_adjacencies_for_face(
                    voxels,
                    lower_chunk.data_offset_if_non_uniform().unwrap(),
                    dim,
                    Side::Upper,
                );

                lower_chunk.mark_upper_face_as_unobscured(dim);
            }
            (Self::Empty, Self::Uniform(_)) => {
                let upper_chunk = &mut chunks[upper_chunk_idx.unwrap()];
                upper_chunk.convert_to_non_uniform_if_uniform(voxels);

                Self::remove_all_outward_adjacencies_for_face(
                    voxels,
                    upper_chunk.data_offset_if_non_uniform().unwrap(),
                    dim,
                    Side::Lower,
                );

                upper_chunk.mark_lower_face_as_unobscured(dim);
            }
            // If one is non-uniform and the other is empty, we need to clear
            // the adjacencies of the non-uniform chunk with the empty chunk, as
            // well as mark the adjoining face of the non-homogeneous chunk as
            // unobscured
            (
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: lower_chunk_data_offset,
                    face_distributions: lower_chunk_face_distributions,
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
                }

                chunks[lower_chunk_idx.unwrap()].mark_upper_face_as_unobscured(dim);
            }
            (
                Self::Empty,
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: upper_chunk_data_offset,
                    face_distributions: upper_chunk_face_distributions,
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
                }

                chunks[upper_chunk_idx.unwrap()].mark_lower_face_as_unobscured(dim);
            }
            // If one is non-uniform and the other is uniform, we need to set
            // the adjacencies of the non-uniform chunk with the uniform chunk,
            // and if the adjoining face of the non-uniform chunk is not full,
            // we must convert the uniform chunk to non-uniform and update its
            // adjacencies as well. We also need to mark the adjoining face of
            // the non-homogeneous chunk as obscured, and potentially the
            // adjoining face of the uniform one as unobscured.
            (
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: lower_chunk_data_offset,
                    face_distributions: lower_chunk_face_distributions,
                    ..
                }),
                Self::Uniform(_),
            ) => {
                let lower_chunk_face_distribution = lower_chunk_face_distributions[dim.idx()][1];

                if lower_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    Self::add_all_outward_adjacencies_for_face(
                        voxels,
                        lower_chunk_data_offset,
                        dim,
                        Side::Upper,
                    );
                }

                chunks[lower_chunk_idx.unwrap()].mark_upper_face_as_obscured(dim);

                match lower_chunk_face_distribution {
                    FaceVoxelDistribution::Full => {}
                    FaceVoxelDistribution::Empty => {
                        let upper_chunk = &mut chunks[upper_chunk_idx.unwrap()];
                        upper_chunk.convert_to_non_uniform_if_uniform(voxels);

                        Self::remove_all_outward_adjacencies_for_face(
                            voxels,
                            upper_chunk.data_offset_if_non_uniform().unwrap(),
                            dim,
                            Side::Lower,
                        );

                        upper_chunk.mark_lower_face_as_unobscured(dim);
                    }
                    FaceVoxelDistribution::Mixed => {
                        let upper_chunk = &mut chunks[upper_chunk_idx.unwrap()];
                        upper_chunk.convert_to_non_uniform_if_uniform(voxels);

                        Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                            voxels,
                            upper_chunk.data_offset_if_non_uniform().unwrap(),
                            lower_chunk_data_offset,
                            dim,
                            Side::Lower,
                        );

                        upper_chunk.mark_lower_face_as_unobscured(dim);
                    }
                }
            }
            (
                Self::Uniform(_),
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: upper_chunk_data_offset,
                    face_distributions: upper_chunk_face_distributions,
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
                }

                chunks[upper_chunk_idx.unwrap()].mark_lower_face_as_obscured(dim);

                match upper_chunk_face_distribution {
                    FaceVoxelDistribution::Full => {}
                    FaceVoxelDistribution::Empty => {
                        let lower_chunk = &mut chunks[lower_chunk_idx.unwrap()];
                        lower_chunk.convert_to_non_uniform_if_uniform(voxels);

                        Self::remove_all_outward_adjacencies_for_face(
                            voxels,
                            lower_chunk.data_offset_if_non_uniform().unwrap(),
                            dim,
                            Side::Upper,
                        );

                        lower_chunk.mark_upper_face_as_unobscured(dim);
                    }
                    FaceVoxelDistribution::Mixed => {
                        let lower_chunk = &mut chunks[lower_chunk_idx.unwrap()];
                        lower_chunk.convert_to_non_uniform_if_uniform(voxels);

                        Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                            voxels,
                            lower_chunk.data_offset_if_non_uniform().unwrap(),
                            upper_chunk_data_offset,
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
                    ..
                }),
                Self::NonUniform(NonUniformVoxelChunk {
                    data_offset: upper_chunk_data_offset,
                    face_distributions: upper_chunk_face_distributions,
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
                        }
                        FaceVoxelDistribution::Full => {
                            Self::add_all_outward_adjacencies_for_face(
                                voxels,
                                lower_chunk_data_offset,
                                dim,
                                Side::Upper,
                            );
                        }
                        FaceVoxelDistribution::Mixed => {
                            Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                                voxels,
                                lower_chunk_data_offset,
                                upper_chunk_data_offset,
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
                        }
                        FaceVoxelDistribution::Full => {
                            Self::add_all_outward_adjacencies_for_face(
                                voxels,
                                upper_chunk_data_offset,
                                dim,
                                Side::Lower,
                            );
                        }
                        FaceVoxelDistribution::Mixed => {
                            Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                                voxels,
                                upper_chunk_data_offset,
                                lower_chunk_data_offset,
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

    fn convert_to_non_uniform_if_uniform(&mut self, voxels: &mut Vec<Voxel>) {
        if let &mut Self::Uniform(voxel) = self {
            let start_voxel_idx = voxels.len();
            voxels.reserve(CHUNK_VOXEL_COUNT);
            voxels.extend(iter::repeat(voxel).take(CHUNK_VOXEL_COUNT));
            *self = Self::NonUniform(NonUniformVoxelChunk {
                data_offset: chunk_data_offset_from_start_voxel_idx(start_voxel_idx),
                face_distributions: [[FaceVoxelDistribution::Full; 2]; 3],
                flags: VoxelChunkFlags::fully_obscured(),
            });
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
        let start_voxel_idx = chunk_start_voxel_idx(data_offset);
        let chunk_voxels = &mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT];

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
        current_chunk_data_offset: u32,
        adjacent_chunk_data_offset: u32,
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
                    }
                }
            },
        );
    }
}

impl FaceEmptyCounts {
    const fn zero() -> Self {
        Self([[0; 2]; 3])
    }

    fn increment_x_dn(&mut self) {
        self.0[0][0] += 1;
    }
    fn increment_x_up(&mut self) {
        self.0[0][1] += 1;
    }
    fn increment_y_dn(&mut self) {
        self.0[1][0] += 1;
    }
    fn increment_y_up(&mut self) {
        self.0[1][1] += 1;
    }
    fn increment_z_dn(&mut self) {
        self.0[2][0] += 1;
    }
    fn increment_z_up(&mut self) {
        self.0[2][1] += 1;
    }

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

    fn map<T>(&self, f: &impl Fn(usize) -> T) -> [[T; 2]; 3] {
        self.0.map(|counts| counts.map(f))
    }
}

impl VoxelChunkFlags {
    const fn fully_obscured() -> Self {
        Self::IS_OBSCURED_X_DN
            .union(Self::IS_OBSCURED_Y_DN)
            .union(Self::IS_OBSCURED_Z_DN)
            .union(Self::IS_OBSCURED_X_UP)
            .union(Self::IS_OBSCURED_Y_UP)
            .union(Self::IS_OBSCURED_Z_UP)
    }

    fn has_exposed_face(&self) -> bool {
        !self.contains(Self::fully_obscured())
    }

    fn mark_lower_face_as_obscured(&mut self, dim: Dimension) {
        self.insert(Self::from_bits_retain(1 << dim as u8));
    }

    fn mark_upper_face_as_obscured(&mut self, dim: Dimension) {
        self.insert(Self::from_bits_retain(1 << (3 + dim as u8)));
    }

    fn mark_lower_face_as_unobscured(&mut self, dim: Dimension) {
        self.remove(Self::from_bits_retain(1 << dim as u8));
    }

    fn mark_upper_face_as_unobscured(&mut self, dim: Dimension) {
        self.remove(Self::from_bits_retain(1 << (3 + dim as u8)));
    }
}

impl ExposedVoxelChunk {
    fn new(chunk_indices: [usize; 3], flags: VoxelChunkFlags) -> Self {
        Self {
            chunk_indices,
            flags,
        }
    }

    /// Returns the indices of the voxel chunk in the object's chunk grid.
    pub fn chunk_indices(&self) -> &[usize; 3] {
        &self.chunk_indices
    }

    /// Returns the flags for the voxel chunk.
    pub fn flags(&self) -> VoxelChunkFlags {
        self.flags
    }

    pub fn lower_voxel_indices(&self) -> [usize; 3] {
        [
            self.chunk_indices[0] * CHUNK_SIZE,
            self.chunk_indices[1] * CHUNK_SIZE,
            self.chunk_indices[2] * CHUNK_SIZE,
        ]
    }

    pub fn upper_voxel_indices(&self) -> [usize; 3] {
        [
            self.chunk_indices[0] * CHUNK_SIZE + CHUNK_SIZE - 1,
            self.chunk_indices[1] * CHUNK_SIZE + CHUNK_SIZE - 1,
            self.chunk_indices[2] * CHUNK_SIZE + CHUNK_SIZE - 1,
        ]
    }
}

const fn chunk_start_voxel_idx(data_offset: u32) -> usize {
    (data_offset as usize) << (3 * LOG2_CHUNK_SIZE)
}

const fn chunk_data_offset_from_start_voxel_idx(start_voxel_idx: usize) -> u32 {
    (start_voxel_idx >> (3 * LOG2_CHUNK_SIZE)) as u32
}

/// Computes the index into a chunk's flattened voxel grid of the voxel at the
/// given indices in the parent object's voxel grid.
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
const fn linear_voxel_idx_within_chunk(voxel_indices: &[usize; 3]) -> usize {
    (voxel_indices[0] << (2 * LOG2_CHUNK_SIZE))
        + (voxel_indices[1] << LOG2_CHUNK_SIZE)
        + voxel_indices[2]
}

/// Computes the 3D index into a chunk's voxel grid for the voxel with the
/// given linear index into the flattened version of the chunk's voxel grid.
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

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::voxel::generation::fuzzing::ArbitrarySDFVoxelGenerator;

    pub fn fuzz_test_voxel_object_generation(generator: ArbitrarySDFVoxelGenerator) {
        if let Some(object) = ChunkedVoxelObject::generate(&generator) {
            object.validate_adjacencies();
            object.validate_chunk_obscuredness();
            object.validate_sdf();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::voxel_types::VoxelType;
    use approx::assert_abs_diff_eq;

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

    impl VoxelGenerator for OffsetBoxVoxelGenerator {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            [
                self.offset[0] + self.shape[0],
                self.offset[1] + self.shape[1],
                self.offset[2] + self.shape[2],
            ]
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel {
            if i >= self.offset[0]
                && i < self.offset[0] + self.shape[0]
                && j >= self.offset[1]
                && j < self.offset[1] + self.shape[1]
                && k >= self.offset[2]
                && k < self.offset[2] + self.shape[2]
            {
                self.voxel
            } else {
                Voxel::maximally_outside()
            }
        }
    }

    impl<const N: usize> VoxelGenerator for ManualVoxelGenerator<N> {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            [self.offset[0] + N, self.offset[1] + N, self.offset[2] + N]
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel {
            if i >= self.offset[0]
                && i < self.offset[0] + N
                && j >= self.offset[1]
                && j < self.offset[1] + N
                && k >= self.offset[2]
                && k < self.offset[2] + N
                && self.voxels[i - self.offset[0]][j - self.offset[1]][k - self.offset[2]] != 0
            {
                Voxel::maximally_inside(VoxelType::default())
            } else {
                Voxel::maximally_outside()
            }
        }
    }

    #[test]
    fn should_yield_none_when_generating_object_with_empty_grid() {
        assert!(ChunkedVoxelObject::generate_without_adjacencies(
            &OffsetBoxVoxelGenerator::with_default([0; 3])
        )
        .is_none());
    }

    #[test]
    fn should_yield_none_when_generating_object_of_empty_voxels() {
        assert!(ChunkedVoxelObject::generate_without_adjacencies(
            &OffsetBoxVoxelGenerator::single_empty()
        )
        .is_none());
        assert!(
            ChunkedVoxelObject::generate_without_adjacencies(&OffsetBoxVoxelGenerator::empty([
                2, 3, 4
            ]))
            .is_none()
        );
    }

    #[test]
    fn should_generate_object_with_single_voxel() {
        let generator = OffsetBoxVoxelGenerator::single_default();
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
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
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
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
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
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
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
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
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
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

        let object = ChunkedVoxelObject::generate(&generator).unwrap();

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

        let object = ChunkedVoxelObject::generate(&generator).unwrap();

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

        let object = ChunkedVoxelObject::generate(&generator).unwrap();

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
    fn should_compute_correct_adjacencies_for_single_voxel() {
        let generator = OffsetBoxVoxelGenerator::with_default([1; 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
    }

    #[test]
    fn should_compute_correct_adjacencies_for_single_chunk() {
        let generator = OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
    }

    #[test]
    fn should_compute_correct_adjacencies_for_barely_two_chunks() {
        let generator =
            OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE + 1, CHUNK_SIZE, CHUNK_SIZE]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator =
            OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE, CHUNK_SIZE + 1, CHUNK_SIZE]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator =
            OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE + 1]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();
    }

    #[test]
    fn should_compute_correct_adjacencies_with_column_taking_barely_two_chunks() {
        let generator = OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE + 1, 1, 1]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator = OffsetBoxVoxelGenerator::with_default([1, CHUNK_SIZE + 1, 1]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_adjacencies();
        object.validate_chunk_obscuredness();

        let generator = OffsetBoxVoxelGenerator::with_default([1, 1, CHUNK_SIZE + 1]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
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
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(aabb.lower_corner(), &point![0.0, 0.0, 0.0]);
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            // The occupied voxel range has chunk granularity, so the AABB will never be smaller
            // than a single chunk
            &point![
                generator.voxel_extent() * CHUNK_SIZE as f64,
                generator.voxel_extent() * CHUNK_SIZE as f64,
                generator.voxel_extent() * CHUNK_SIZE as f64
            ]
        );
    }

    #[test]
    fn should_compute_correct_aabb_for_single_chunk() {
        let generator = OffsetBoxVoxelGenerator::with_default([CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(aabb.lower_corner(), &point![0.0, 0.0, 0.0]);
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &point![
                generator.voxel_extent() * CHUNK_SIZE as f64,
                generator.voxel_extent() * CHUNK_SIZE as f64,
                generator.voxel_extent() * CHUNK_SIZE as f64
            ]
        );
    }

    #[test]
    fn should_compute_correct_aabb_for_different_numbers_of_chunks_along_each_axis() {
        let generator =
            OffsetBoxVoxelGenerator::with_default([2 * CHUNK_SIZE, 3 * CHUNK_SIZE, 4 * CHUNK_SIZE]);
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(aabb.lower_corner(), &point![0.0, 0.0, 0.0]);
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &point![
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f64,
                generator.voxel_extent() * (3 * CHUNK_SIZE) as f64,
                generator.voxel_extent() * (4 * CHUNK_SIZE) as f64
            ]
        );
    }

    #[test]
    fn should_compute_correct_aabb_for_offset_chunk() {
        let generator =
            OffsetBoxVoxelGenerator::offset_with_default([CHUNK_SIZE; 3], [CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
        let aabb = object.compute_aabb();
        assert_abs_diff_eq!(
            aabb.lower_corner(),
            &point![
                generator.voxel_extent() * CHUNK_SIZE as f64,
                generator.voxel_extent() * CHUNK_SIZE as f64,
                generator.voxel_extent() * CHUNK_SIZE as f64
            ]
        );
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &point![
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f64,
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f64,
                generator.voxel_extent() * (2 * CHUNK_SIZE) as f64
            ]
        );
    }
}
