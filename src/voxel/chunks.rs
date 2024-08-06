//! Chunked representation of voxel objects.

use crate::{
    num::Float,
    voxel::{VoxelGenerator, VoxelType},
};
use bitflags::bitflags;
use std::{iter, ops::Range};

/// An object represented by a grid of voxels.
///
/// The grid is subdivided into cubic chunks that are [`CHUNK_SIZE`] voxels
/// across. The grid of chunks is further subdivided into superchunks that are
/// [`SUPERCHUNK_SIZE`] chunks across. The full grid for the object spans the
/// same whole number of superchunks along each axis.
///
/// Uniform voxel information is pulled up to the coarsest possible level of
/// detail. For example, an empty chunk does not store any information on the
/// voxel level, and an empty superchunk does not store any information on the
/// chunk level. Furthermore, a chunk or superchunk where all voxels contain the
/// exact same information only stores that single voxel.
#[derive(Clone, Debug)]
pub struct ChunkedVoxelObject {
    voxel_extent: f64,
    n_superchunks_per_axis: usize,
    occupied_chunks: [Range<usize>; 3],
    superchunks: Vec<VoxelSuperchunk>,
    chunks: Vec<VoxelChunk>,
    voxels: Vec<Voxel>,
}

/// A voxel, which may either be be empty or filled with a material with
/// specific properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voxel {
    property_id: PropertyID,
    flags: VoxelFlags,
}

/// Identifier for predefined set of voxel properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropertyID(u8);

bitflags! {
    /// Bitflag encoding a set of possible states for a voxel.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VoxelFlags: u8 {
        /// The voxel is empty.
        const IS_EMPTY          = 1 << 0;
        /// The voxel has an adjacent non-empty voxel in the negative
        /// x-direction.
        const HAS_ADJACENT_X_DN = 1 << 2;
        /// The voxel has an adjacent non-empty voxel in the negative
        /// y-direction.
        const HAS_ADJACENT_Y_DN = 1 << 3;
        /// The voxel has an adjacent non-empty voxel in the negative
        /// z-direction.
        const HAS_ADJACENT_Z_DN = 1 << 4;
        /// The voxel has an adjacent non-empty voxel in the positive
        /// x-direction.
        const HAS_ADJACENT_X_UP = 1 << 5;
        /// The voxel has an adjacent non-empty voxel in the positive
        /// y-direction.
        const HAS_ADJACENT_Y_UP = 1 << 6;
        /// The voxel has an adjacent non-empty voxel in the positive
        /// z-direction.
        const HAS_ADJACENT_Z_UP = 1 << 7;
    }
}

/// A superchunk representing a cubic grid of voxel chunks. It has three
/// representations:
/// - Empty: The superchunk contains no voxels.
/// - Uniform: The superchunk is fully packed with voxels carrying the exact
///   same information. Only the single representative voxel is stored.
/// - Non-uniform: The superchunk is not full packed and/or contains a mix of
///   voxels with different information. The chunks comprising the non-uniform
///   superchunk are stored in the parent [`ChunkedVoxelObject`], and the
///   superchunk stores the index to its first chunk as well as
///   information on the distribution of voxels across the faces of the
///   superchunk.
#[derive(Clone, Debug)]
enum VoxelSuperchunk {
    Empty,
    Uniform(Voxel),
    NonUniform {
        start_chunk_idx: usize,
        face_distributions: [[FaceVoxelDistribution; 2]; 3],
    },
}

/// A chunk representing a cubic grid of voxels. It has three representations:
/// - Empty: The chunk contains no voxels.
/// - Uniform: The chunk is fully packed with voxels carrying the exact same
///   information. Only the single representative voxel is stored.
/// - Non-uniform: The chunk is not full packed and/or contains a mix of voxels
///   with different information. The voxels comprising the non-uniform chunk
///   are stored in the parent [`ChunkedVoxelObject`], and the chunk stores the
///   index to its first voxel as well as information on the distribution of
///   voxels across the faces of the chunk.
#[derive(Clone, Debug)]
enum VoxelChunk {
    Empty,
    Uniform(Voxel),
    NonUniform {
        start_voxel_idx: usize,
        face_distributions: [[FaceVoxelDistribution; 2]; 3],
    },
}

/// Information about the distribution of voxels across a specific face of a
/// chunk or superchunk.
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

/// Helper struct for keeping track of the number of empty voxels on each face
/// of a chunk or superchunk.
struct FaceEmptyCounts([[usize; 2]; 3]);

/// A 3D spatial dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dimension {
    X = 0,
    Y = 1,
    Z = 2,
}

/// A specific face of a chunk or superchunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Face {
    LowerX = 0,
    UpperX = 1,
    LowerY = 2,
    UpperY = 3,
    LowerZ = 4,
    UpperZ = 5,
}

/// A generalized index referring to a chunk or superchunk that may not be
/// stored explicitly in the parent [`ChunkedVoxelObject`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChunkIndex {
    /// The chunk or superchunk is not stored anywhere, but it is empty.
    AbsentEmpty,
    /// The chunk or superchunk is not stored anywhere, but it is uniformly
    /// filled with the given voxel.
    AbsentUniform(Voxel),
    /// The chunk or superchunk is stored at the given index in the parent
    /// [`ChunkedVoxelObject`].
    Present(usize),
}

const LOG2_CHUNK_SIZE: usize = 4;
const LOG2_SUPERCHUNK_SIZE: usize = 3;

/// The number of voxels across a cubic voxel chunk. It is always a power of
/// two.
pub const CHUNK_SIZE: usize = 1 << LOG2_CHUNK_SIZE;
const CHUNK_SIZE_SQUARED: usize = CHUNK_SIZE.pow(2);
/// The total number of voxels comprising each chunk.
const CHUNK_VOXEL_COUNT: usize = CHUNK_SIZE.pow(3);

/// The number of chunks across a cubic superchunk. It is always a power of two.
pub const SUPERCHUNK_SIZE: usize = 1 << LOG2_SUPERCHUNK_SIZE;
const SUPERCHUNK_SIZE_SQUARED: usize = SUPERCHUNK_SIZE.pow(2);
/// The number of voxels across a cubic superchunk.
const SUPERCHUNK_SIZE_IN_VOXELS: usize = SUPERCHUNK_SIZE * CHUNK_SIZE;
const SUPERCHUNK_SIZE_IN_VOXELS_SQUARED: usize = SUPERCHUNK_SIZE_IN_VOXELS.pow(2);
/// The total number of chunks comprising each superchunk.
const SUPERCHUNK_CHUNK_COUNT: usize = SUPERCHUNK_SIZE.pow(3);

const SUPERCHUNK_IDX_SHIFT: usize = LOG2_CHUNK_SIZE + LOG2_SUPERCHUNK_SIZE;
const CHUNK_IDX_SHIFT: usize = LOG2_CHUNK_SIZE;
const CHUNK_IDX_MASK: usize = (1 << LOG2_SUPERCHUNK_SIZE) - 1;
const VOXEL_IDX_MASK: usize = (1 << LOG2_CHUNK_SIZE) - 1;

#[allow(clippy::reversed_empty_ranges)]
const REVERSED_MAX_RANGE: Range<usize> = usize::MAX..usize::MIN;

impl ChunkedVoxelObject {
    /// Generates a new `ChunkedVoxelObject` using the given [`VoxelGenerator`].
    /// Returns [`None`] if the resulting object would not contain any voxels.
    pub fn generate<G, F>(generator: &G) -> Option<Self>
    where
        G: VoxelGenerator<F>,
        F: Float,
    {
        let generator_grid_shape = generator.grid_shape();

        if generator_grid_shape.iter().any(|&dim| dim == 0) {
            return None;
        }

        let n_superchunks_per_axis = generator_grid_shape
            .iter()
            .map(|size| size.div_ceil(SUPERCHUNK_SIZE_IN_VOXELS))
            .max()
            .unwrap();

        let mut superchunks = Vec::with_capacity(n_superchunks_per_axis.pow(3));

        let mut chunks = Vec::new();
        let mut voxels = Vec::new();

        let mut occupied_chunks_i = REVERSED_MAX_RANGE;
        let mut occupied_chunks_j = REVERSED_MAX_RANGE;
        let mut occupied_chunks_k = REVERSED_MAX_RANGE;

        for superchunk_i in 0..n_superchunks_per_axis {
            for superchunk_j in 0..n_superchunks_per_axis {
                for superchunk_k in 0..n_superchunks_per_axis {
                    let (superchunk, occupied_chunks) = VoxelSuperchunk::generate(
                        &mut chunks,
                        &mut voxels,
                        generator,
                        [superchunk_i, superchunk_j, superchunk_k],
                    );

                    occupied_chunks_i.start = occupied_chunks_i.start.min(occupied_chunks[0].start);
                    occupied_chunks_i.end = occupied_chunks_i.end.max(occupied_chunks[0].end);
                    occupied_chunks_j.start = occupied_chunks_j.start.min(occupied_chunks[1].start);
                    occupied_chunks_j.end = occupied_chunks_j.end.max(occupied_chunks[1].end);
                    occupied_chunks_k.start = occupied_chunks_k.start.min(occupied_chunks[2].start);
                    occupied_chunks_k.end = occupied_chunks_k.end.max(occupied_chunks[2].end);

                    superchunks.push(superchunk);
                }
            }
        }

        if superchunks.iter().all(VoxelSuperchunk::is_empty) {
            return None;
        }

        let occupied_chunks = [occupied_chunks_i, occupied_chunks_j, occupied_chunks_k];

        Some(Self {
            voxel_extent: generator.voxel_extent().to_f64().unwrap(),
            n_superchunks_per_axis,
            occupied_chunks,
            superchunks,
            chunks,
            voxels,
        })
    }

    /// Returns the extent of single voxel in the object.
    pub fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    /// Returns the number of superchunks along each axis of the object's voxel
    /// grid.
    pub fn n_superchunks_per_axis(&self) -> usize {
        self.n_superchunks_per_axis
    }

    /// Returns the number of voxels along each axis of the object's voxel grid.
    pub fn full_grid_size(&self) -> usize {
        self.n_superchunks_per_axis * SUPERCHUNK_SIZE_IN_VOXELS
    }

    /// Returns the range of indices along the given axis of the object's voxel
    /// grid that may contain non-empty voxels.
    pub fn occupied_range(&self, axis: usize) -> Range<usize> {
        self.occupied_chunks[axis].start * CHUNK_SIZE..self.occupied_chunks[axis].end * CHUNK_SIZE
    }

    /// Returns the number of voxels (potentially empty) actually stored in the
    /// object (as opposed to the count of voxels the object logically
    /// contains).
    pub fn stored_voxel_count(&self) -> usize {
        self.superchunks
            .iter()
            .map(|superchunk| superchunk.stored_voxel_count(&self.chunks))
            .sum()
    }

    /// Returns a reference to the voxel at the given indices in the object's
    /// voxel grid, or [`None`] if the voxel is empty or the indices are out of
    /// bounds.
    ///
    /// Despite the hierarchical organization of voxels into chunks and
    /// superchunks, this lookup is relatively efficient because we can perform
    /// simple bit manipulations to determine the superchunk and chunk
    /// containing the voxel.
    pub fn get_voxel(&self, i: usize, j: usize, k: usize) -> Option<&Voxel> {
        let superchunk_idx = self.linear_superchunk_idx_from_object_voxel_indices(i, j, k);
        let superchunk = self.superchunks.get(superchunk_idx)?;
        match &superchunk {
            VoxelSuperchunk::Empty => None,
            VoxelSuperchunk::Uniform(voxel) => Some(voxel),
            VoxelSuperchunk::NonUniform {
                start_chunk_idx, ..
            } => {
                let chunk_idx = start_chunk_idx
                    + linear_chunk_idx_within_superchunk_from_object_voxel_indices(i, j, k);
                let chunk = &self.chunks[chunk_idx];
                match &chunk {
                    VoxelChunk::Empty => None,
                    VoxelChunk::Uniform(voxel) => Some(voxel),
                    VoxelChunk::NonUniform {
                        start_voxel_idx, ..
                    } => {
                        let voxel_idx = start_voxel_idx
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
        }
    }

    /// Updates the adjacency [`VoxelFlags`] for each voxel in the object
    /// according to which of their six neighbor voxels are present.
    pub fn update_adjacencies(&mut self) {
        for chunk in &self.chunks {
            chunk.update_internal_adjacencies(self.voxels.as_mut_slice());
        }
        self.update_all_chunk_boundary_adjacencies();
    }

    /// Validates the adjacency [`VoxelFlags`] computed by the efficient
    /// [`Self::update_adjacencies`] method by performing a simple brute-force
    /// iteration over all voxels and checking their neighbors.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_adjacencies(&self) {
        let mut invalid_missing_flags = Vec::new();
        let mut invalid_present_flags = Vec::new();

        for i in self.occupied_range(0) {
            for j in self.occupied_range(1) {
                for k in self.occupied_range(2) {
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

                    let voxel = self.get_voxel(i, j, k).copied().unwrap_or(Voxel::empty());

                    let adjacent_voxel_x_up = self
                        .get_voxel(i + 1, j, k)
                        .copied()
                        .unwrap_or(Voxel::empty());
                    let adjacent_voxel_y_up = self
                        .get_voxel(i, j + 1, k)
                        .copied()
                        .unwrap_or(Voxel::empty());
                    let adjacent_voxel_z_up = self
                        .get_voxel(i, j, k + 1)
                        .copied()
                        .unwrap_or(Voxel::empty());

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

        for j in self.occupied_range(1) {
            for k in self.occupied_range(2) {
                if let Some(voxel) = self.get_voxel(0, j, k) {
                    if voxel.flags().contains(VoxelFlags::HAS_ADJACENT_X_DN) {
                        invalid_present_flags.push(([0, j, k], VoxelFlags::HAS_ADJACENT_X_DN));
                    }
                }
            }
        }
        for i in self.occupied_range(0) {
            for k in self.occupied_range(2) {
                if let Some(voxel) = self.get_voxel(i, 0, k) {
                    if voxel.flags().contains(VoxelFlags::HAS_ADJACENT_Y_DN) {
                        invalid_present_flags.push(([i, 0, k], VoxelFlags::HAS_ADJACENT_Y_DN));
                    }
                }
            }
        }
        for i in self.occupied_range(0) {
            for j in self.occupied_range(1) {
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

    fn update_all_chunk_boundary_adjacencies(&mut self) {
        let mut superchunk_idx = 0;

        for superchunk_i in 0..self.n_superchunks_per_axis {
            for superchunk_j in 0..self.n_superchunks_per_axis {
                for superchunk_k in 0..self.n_superchunks_per_axis {
                    for (adjacent_superchunk_indices, dim) in [
                        ([superchunk_i + 1, superchunk_j, superchunk_k], Dimension::X),
                        ([superchunk_i, superchunk_j + 1, superchunk_k], Dimension::Y),
                        ([superchunk_i, superchunk_j, superchunk_k + 1], Dimension::Z),
                    ] {
                        let adjacent_superchunk_idx =
                            self.linear_superchunk_idx(&adjacent_superchunk_indices);

                        let lower_superchunk_idx = ChunkIndex::Present(superchunk_idx);

                        let upper_superchunk_idx = if adjacent_superchunk_indices[dim.idx()]
                            < self.n_superchunks_per_axis
                        {
                            ChunkIndex::Present(adjacent_superchunk_idx)
                        } else {
                            ChunkIndex::AbsentEmpty
                        };

                        VoxelSuperchunk::update_mutual_face_adjacencies(
                            self.superchunks.as_mut_slice(),
                            &mut self.chunks,
                            &mut self.voxels,
                            lower_superchunk_idx,
                            upper_superchunk_idx,
                            dim,
                        );
                    }

                    self.superchunks[superchunk_idx].update_internal_chunk_boundary_adjacencies(
                        self.chunks.as_mut_slice(),
                        &mut self.voxels,
                    );

                    superchunk_idx += 1;
                }
            }
        }

        // Handle lower faces of the full object, since these are not included
        // in the loop above
        for superchunk_n in 0..self.n_superchunks_per_axis {
            for superchunk_m in 0..self.n_superchunks_per_axis {
                for (superchunk_indices, dim) in [
                    ([0, superchunk_n, superchunk_m], Dimension::X),
                    ([superchunk_n, 0, superchunk_m], Dimension::Y),
                    ([superchunk_n, superchunk_m, 0], Dimension::Z),
                ] {
                    let superchunk_idx = self.linear_superchunk_idx(&superchunk_indices);

                    VoxelSuperchunk::update_mutual_face_adjacencies(
                        self.superchunks.as_mut_slice(),
                        &mut self.chunks,
                        &mut self.voxels,
                        ChunkIndex::AbsentEmpty,
                        ChunkIndex::Present(superchunk_idx),
                        dim,
                    );
                }
            }
        }
    }

    /// Computes the index in `self.superchunks` of the superchunk containing
    /// the voxel at the given indices into the object's voxel grid.
    fn linear_superchunk_idx_from_object_voxel_indices(
        &self,
        i: usize,
        j: usize,
        k: usize,
    ) -> usize {
        let superchunk_indices = superchunk_indices_from_object_voxel_indices(i, j, k);
        self.linear_superchunk_idx(&superchunk_indices)
    }

    /// Computes the index in `self.superchunks` of the superchunk with the
    /// given 3D index in the object's superchunk grid.
    fn linear_superchunk_idx(&self, superchunk_indices: &[usize; 3]) -> usize {
        superchunk_indices[0] * self.n_superchunks_per_axis * self.n_superchunks_per_axis
            + superchunk_indices[1] * self.n_superchunks_per_axis
            + superchunk_indices[2]
    }
}

impl VoxelSuperchunk {
    fn generate<G, F>(
        chunks: &mut Vec<VoxelChunk>,
        voxels: &mut Vec<Voxel>,
        generator: &G,
        superchunk_indices: [usize; 3],
    ) -> (Self, [Range<usize>; 3])
    where
        G: VoxelGenerator<F>,
        F: Float,
    {
        let mut first_voxel: Option<Voxel> = None;
        let mut is_uniform = true;

        let start_chunk_idx = chunks.len();
        chunks.reserve(SUPERCHUNK_CHUNK_COUNT);

        // Note: These are global chunk indices, not the chunk indices within
        // the current superchunk
        let start_chunk_indices = superchunk_indices.map(|idx| idx * SUPERCHUNK_SIZE);

        let mut face_empty_counts = FaceEmptyCounts::zero();

        let mut occupied_chunks_i = REVERSED_MAX_RANGE;
        let mut occupied_chunks_j = REVERSED_MAX_RANGE;
        let mut occupied_chunks_k = REVERSED_MAX_RANGE;

        let range_i = start_chunk_indices[0]..start_chunk_indices[0] + SUPERCHUNK_SIZE;
        let range_j = start_chunk_indices[1]..start_chunk_indices[1] + SUPERCHUNK_SIZE;
        let range_k = start_chunk_indices[2]..start_chunk_indices[2] + SUPERCHUNK_SIZE;

        for chunk_i in range_i.clone() {
            for chunk_j in range_j.clone() {
                for chunk_k in range_k.clone() {
                    let (chunk, chunk_face_empty_counts) =
                        VoxelChunk::generate(voxels, generator, [chunk_i, chunk_j, chunk_k]);

                    if is_uniform {
                        match (&first_voxel, &chunk) {
                            (Some(first_voxel), VoxelChunk::Empty) => {
                                is_uniform = first_voxel.is_empty();
                            }
                            (Some(first_voxel), VoxelChunk::Uniform(voxel)) => {
                                is_uniform = first_voxel == voxel;
                            }
                            (_, VoxelChunk::NonUniform { .. }) => {
                                is_uniform = false;
                            }
                            (None, VoxelChunk::Empty) => {
                                first_voxel = Some(Voxel::empty());
                            }
                            (None, VoxelChunk::Uniform(voxel)) => {
                                first_voxel = Some(*voxel);
                            }
                        }
                    }

                    if !chunk.is_empty() {
                        occupied_chunks_i.start = occupied_chunks_i.start.min(chunk_i);
                        occupied_chunks_i.end = occupied_chunks_i.end.max(chunk_i + 1);
                        occupied_chunks_j.start = occupied_chunks_j.start.min(chunk_j);
                        occupied_chunks_j.end = occupied_chunks_j.end.max(chunk_j + 1);
                        occupied_chunks_k.start = occupied_chunks_k.start.min(chunk_k);
                        occupied_chunks_k.end = occupied_chunks_k.end.max(chunk_k + 1);
                    }

                    if chunk_i == range_i.start {
                        face_empty_counts.add_x_dn(&chunk_face_empty_counts);
                    } else if chunk_i == range_i.end - 1 {
                        face_empty_counts.add_x_up(&chunk_face_empty_counts);
                    }
                    if chunk_j == range_j.start {
                        face_empty_counts.add_y_dn(&chunk_face_empty_counts);
                    } else if chunk_j == range_j.end - 1 {
                        face_empty_counts.add_y_up(&chunk_face_empty_counts);
                    }
                    if chunk_k == range_k.start {
                        face_empty_counts.add_z_dn(&chunk_face_empty_counts);
                    } else if chunk_k == range_k.end - 1 {
                        face_empty_counts.add_z_up(&chunk_face_empty_counts);
                    }

                    chunks.push(chunk);
                }
            }
        }

        let occupied_chunks = [occupied_chunks_i, occupied_chunks_j, occupied_chunks_k];

        if is_uniform {
            chunks.truncate(start_chunk_idx);

            let mut first_voxel = first_voxel.unwrap();
            first_voxel.add_flags(VoxelFlags::full_adjacency());

            (
                if first_voxel.is_empty() {
                    Self::Empty
                } else {
                    Self::Uniform(first_voxel)
                },
                occupied_chunks,
            )
        } else {
            let face_distributions =
                face_empty_counts.to_face_distributions(SUPERCHUNK_SIZE_IN_VOXELS_SQUARED);
            (
                Self::NonUniform {
                    start_chunk_idx,
                    face_distributions,
                },
                occupied_chunks,
            )
        }
    }

    const fn uniform(voxel: Voxel) -> Self {
        Self::Uniform(voxel)
    }

    const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    const fn start_chunk_idx(&self) -> ChunkIndex {
        match self {
            Self::Empty => ChunkIndex::AbsentEmpty,
            Self::Uniform(voxel) => ChunkIndex::AbsentUniform(*voxel),
            Self::NonUniform {
                start_chunk_idx, ..
            } => ChunkIndex::Present(*start_chunk_idx),
        }
    }

    fn stored_voxel_count(&self, chunks: &[VoxelChunk]) -> usize {
        match self {
            Self::Empty => 0,
            Self::Uniform(_) => 1,
            &Self::NonUniform {
                start_chunk_idx, ..
            } => chunks[start_chunk_idx..start_chunk_idx + SUPERCHUNK_CHUNK_COUNT]
                .iter()
                .map(VoxelChunk::stored_voxel_count)
                .sum(),
        }
    }

    fn update_internal_chunk_boundary_adjacencies(
        &self,
        chunks: &mut [VoxelChunk],
        voxels: &mut Vec<Voxel>,
    ) {
        // We only need to update the internal adjacency if the superchunk is
        // non-uniform
        let start_chunk_idx = if let Self::NonUniform {
            start_chunk_idx, ..
        } = self
        {
            *start_chunk_idx
        } else {
            return;
        };

        // Extract the sub-slice of chunk for this superchunk so that we get
        // out-of-bounds when trying to access chunks outside the superchunk
        let superchunk_chunks =
            &mut chunks[start_chunk_idx..start_chunk_idx + SUPERCHUNK_CHUNK_COUNT];

        for chunk_i in 0..SUPERCHUNK_SIZE {
            for chunk_j in 0..SUPERCHUNK_SIZE {
                for chunk_k in 0..SUPERCHUNK_SIZE {
                    let chunk_idx =
                        linear_chunk_idx_within_superchunk(&[chunk_i, chunk_j, chunk_k]);

                    for (adjacent_chunk_indices, dim) in [
                        ([chunk_i + 1, chunk_j, chunk_k], Dimension::X),
                        ([chunk_i, chunk_j + 1, chunk_k], Dimension::Y),
                        ([chunk_i, chunk_j, chunk_k + 1], Dimension::Z),
                    ] {
                        let adjacent_chunk_idx =
                            linear_chunk_idx_within_superchunk(&adjacent_chunk_indices);

                        if adjacent_chunk_indices[dim.idx()] < SUPERCHUNK_SIZE {
                            VoxelChunk::update_mutual_face_adjacencies(
                                superchunk_chunks,
                                voxels,
                                ChunkIndex::Present(chunk_idx),
                                ChunkIndex::Present(adjacent_chunk_idx),
                                dim,
                            );
                        }
                    }
                }
            }
        }
    }

    fn convert_to_non_uniform_if_uniform(&mut self, chunks: &mut Vec<VoxelChunk>) {
        if let Self::Uniform(voxel) = self {
            let start_chunk_idx = chunks.len();
            chunks.reserve(SUPERCHUNK_CHUNK_COUNT);
            chunks.extend(iter::repeat(VoxelChunk::uniform(*voxel)).take(SUPERCHUNK_CHUNK_COUNT));
            *self = Self::NonUniform {
                start_chunk_idx,
                face_distributions: [[FaceVoxelDistribution::Full; 2]; 3],
            };
        }
    }

    fn update_mutual_face_adjacencies(
        superchunks: &mut [VoxelSuperchunk],
        chunks: &mut Vec<VoxelChunk>,
        voxels: &mut Vec<Voxel>,
        lower_superchunk_idx: ChunkIndex,
        upper_superchunk_idx: ChunkIndex,
        dim: Dimension,
    ) {
        let lower_superchunk = lower_superchunk_idx.to_superchunk(superchunks);
        let upper_superchunk = upper_superchunk_idx.to_superchunk(superchunks);

        match (lower_superchunk, upper_superchunk) {
            // If both superchunks are empty or uniform, there is nothing to do
            (Self::Empty, Self::Empty) | (Self::Uniform(_), Self::Uniform(_)) => {}
            // If one is uniform and the other is empty, we need to convert the
            // uniform superchunk to non-uniform and update its adjacencies with
            // the empty superchunk
            (Self::Uniform(_), Self::Empty) => {
                let lower_superchunk = &mut superchunks[lower_superchunk_idx.unwrap_idx()];
                lower_superchunk.convert_to_non_uniform_if_uniform(chunks);
                Self::update_mutual_outward_adjacencies_for_dim(
                    chunks,
                    voxels,
                    lower_superchunk.start_chunk_idx(),
                    ChunkIndex::AbsentEmpty,
                    dim,
                );
            }
            (Self::Empty, Self::Uniform(_)) => {
                let upper_superchunk = &mut superchunks[upper_superchunk_idx.unwrap_idx()];
                upper_superchunk.convert_to_non_uniform_if_uniform(chunks);
                Self::update_mutual_outward_adjacencies_for_dim(
                    chunks,
                    voxels,
                    ChunkIndex::AbsentEmpty,
                    upper_superchunk.start_chunk_idx(),
                    dim,
                );
            }
            // If one is non-uniform and the other is empty, we need to clear
            // the adjacencies of the non-uniform superchunk with the empty
            // superchunk
            (
                Self::NonUniform {
                    start_chunk_idx: lower_superchunk_start_chunk_idx,
                    face_distributions: lower_superchunk_face_distributions,
                },
                Self::Empty,
            ) => {
                // We can skip the update if there are no voxels on the face
                if lower_superchunk_face_distributions[dim.idx()][1] != FaceVoxelDistribution::Empty
                {
                    Self::update_mutual_outward_adjacencies_for_dim(
                        chunks,
                        voxels,
                        ChunkIndex::Present(lower_superchunk_start_chunk_idx),
                        ChunkIndex::AbsentEmpty,
                        dim,
                    );
                }
            }
            (
                Self::Empty,
                Self::NonUniform {
                    start_chunk_idx: upper_superchunk_start_chunk_idx,
                    face_distributions: upper_superchunk_face_distributions,
                },
            ) => {
                if upper_superchunk_face_distributions[dim.idx()][0] != FaceVoxelDistribution::Empty
                {
                    Self::update_mutual_outward_adjacencies_for_dim(
                        chunks,
                        voxels,
                        ChunkIndex::AbsentEmpty,
                        ChunkIndex::Present(upper_superchunk_start_chunk_idx),
                        dim,
                    );
                }
            }
            // If one is non-uniform and the other is uniform, we need to set
            // the adjacencies of the non-uniform superchunk with the uniform
            // superchunk, and if the adjoining face of the non-uniform
            // superchunk is not full, we must convert the uniform superchunk to
            // non-uniform and update its adjacencies as well
            (
                Self::NonUniform {
                    start_chunk_idx: lower_superchunk_start_chunk_idx,
                    face_distributions: lower_superchunk_face_distributions,
                    ..
                },
                Self::Uniform(voxel),
            ) => {
                let lower_superchunk_face_distribution =
                    lower_superchunk_face_distributions[dim.idx()][1];

                if lower_superchunk_face_distribution != FaceVoxelDistribution::Empty {
                    Self::update_mutual_outward_adjacencies_for_dim(
                        chunks,
                        voxels,
                        ChunkIndex::Present(lower_superchunk_start_chunk_idx),
                        ChunkIndex::AbsentUniform(voxel),
                        dim,
                    );
                }

                match lower_superchunk_face_distribution {
                    FaceVoxelDistribution::Full => {}
                    FaceVoxelDistribution::Empty => {
                        let upper_superchunk = &mut superchunks[upper_superchunk_idx.unwrap_idx()];
                        upper_superchunk.convert_to_non_uniform_if_uniform(chunks);
                        Self::update_mutual_outward_adjacencies_for_dim(
                            chunks,
                            voxels,
                            ChunkIndex::AbsentEmpty,
                            upper_superchunk.start_chunk_idx(),
                            dim,
                        );
                    }
                    FaceVoxelDistribution::Mixed => {
                        let upper_superchunk = &mut superchunks[upper_superchunk_idx.unwrap_idx()];
                        upper_superchunk.convert_to_non_uniform_if_uniform(chunks);
                        Self::update_mutual_outward_adjacencies_for_dim(
                            chunks,
                            voxels,
                            ChunkIndex::Present(lower_superchunk_start_chunk_idx),
                            upper_superchunk.start_chunk_idx(),
                            dim,
                        );
                    }
                }
            }
            (
                Self::Uniform(voxel),
                Self::NonUniform {
                    start_chunk_idx: upper_superchunk_start_chunk_idx,
                    face_distributions: upper_superchunk_face_distributions,
                },
            ) => {
                let upper_superchunk_face_distribution =
                    upper_superchunk_face_distributions[dim.idx()][0];

                if upper_superchunk_face_distribution != FaceVoxelDistribution::Empty {
                    Self::update_mutual_outward_adjacencies_for_dim(
                        chunks,
                        voxels,
                        ChunkIndex::AbsentUniform(voxel),
                        ChunkIndex::Present(upper_superchunk_start_chunk_idx),
                        dim,
                    );
                }

                match upper_superchunk_face_distribution {
                    FaceVoxelDistribution::Full => {}
                    FaceVoxelDistribution::Empty => {
                        let lower_superchunk = &mut superchunks[lower_superchunk_idx.unwrap_idx()];
                        lower_superchunk.convert_to_non_uniform_if_uniform(chunks);
                        Self::update_mutual_outward_adjacencies_for_dim(
                            chunks,
                            voxels,
                            lower_superchunk.start_chunk_idx(),
                            ChunkIndex::AbsentEmpty,
                            dim,
                        );
                    }
                    FaceVoxelDistribution::Mixed => {
                        let lower_superchunk = &mut superchunks[lower_superchunk_idx.unwrap_idx()];
                        lower_superchunk.convert_to_non_uniform_if_uniform(chunks);
                        Self::update_mutual_outward_adjacencies_for_dim(
                            chunks,
                            voxels,
                            lower_superchunk.start_chunk_idx(),
                            ChunkIndex::Present(upper_superchunk_start_chunk_idx),
                            dim,
                        );
                    }
                }
            }
            // If both superchunks are non-uniform, we need to update the
            // adjacencies for both according to their adjoining faces
            (
                Self::NonUniform {
                    start_chunk_idx: lower_superchunk_start_chunk_idx,
                    ..
                },
                Self::NonUniform {
                    start_chunk_idx: upper_superchunk_start_chunk_idx,
                    ..
                },
            ) => {
                Self::update_mutual_outward_adjacencies_for_dim(
                    chunks,
                    voxels,
                    ChunkIndex::Present(lower_superchunk_start_chunk_idx),
                    ChunkIndex::Present(upper_superchunk_start_chunk_idx),
                    dim,
                );
            }
        }
    }

    fn update_mutual_outward_adjacencies_for_dim(
        chunks: &mut [VoxelChunk],
        voxels: &mut Vec<Voxel>,
        lower_superchunk_start_chunk_idx: ChunkIndex,
        upper_superchunk_start_chunk_idx: ChunkIndex,
        dim: Dimension,
    ) {
        let lower_chunk_idx = |chunk_indices| {
            lower_superchunk_start_chunk_idx
                .map_idx(|start_idx| start_idx + linear_chunk_idx_within_superchunk(&chunk_indices))
        };
        let upper_chunk_idx = |chunk_indices| {
            upper_superchunk_start_chunk_idx
                .map_idx(|start_idx| start_idx + linear_chunk_idx_within_superchunk(&chunk_indices))
        };

        match dim {
            Dimension::X => {
                for j in 0..SUPERCHUNK_SIZE {
                    for k in 0..SUPERCHUNK_SIZE {
                        VoxelChunk::update_mutual_face_adjacencies(
                            chunks,
                            voxels,
                            lower_chunk_idx([SUPERCHUNK_SIZE - 1, j, k]),
                            upper_chunk_idx([0, j, k]),
                            Dimension::X,
                        );
                    }
                }
            }
            Dimension::Y => {
                for i in 0..SUPERCHUNK_SIZE {
                    for k in 0..SUPERCHUNK_SIZE {
                        VoxelChunk::update_mutual_face_adjacencies(
                            chunks,
                            voxels,
                            lower_chunk_idx([i, SUPERCHUNK_SIZE - 1, k]),
                            upper_chunk_idx([i, 0, k]),
                            Dimension::Y,
                        );
                    }
                }
            }
            Dimension::Z => {
                for i in 0..SUPERCHUNK_SIZE {
                    for j in 0..SUPERCHUNK_SIZE {
                        VoxelChunk::update_mutual_face_adjacencies(
                            chunks,
                            voxels,
                            lower_chunk_idx([i, j, SUPERCHUNK_SIZE - 1]),
                            upper_chunk_idx([i, j, 0]),
                            Dimension::Z,
                        );
                    }
                }
            }
        }
    }
}

impl VoxelChunk {
    fn generate<G, F>(
        voxels: &mut Vec<Voxel>,
        generator: &G,
        global_chunk_indices: [usize; 3],
    ) -> (Self, FaceEmptyCounts)
    where
        G: VoxelGenerator<F>,
        F: Float,
    {
        let origin = [
            global_chunk_indices[0] * CHUNK_SIZE,
            global_chunk_indices[1] * CHUNK_SIZE,
            global_chunk_indices[2] * CHUNK_SIZE,
        ];

        // Return early if the chunk is completely outside the grid
        if origin
            .iter()
            .zip(generator.grid_shape())
            .any(|(&idx, size)| idx >= size)
        {
            return (Self::Empty, FaceEmptyCounts::same(CHUNK_SIZE_SQUARED));
        }

        let mut first_voxel = generator
            .voxel_at_indices(origin[0], origin[1], origin[2])
            .map_or_else(Voxel::empty, Voxel::new_from_type_without_flags);
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
                    let voxel = generator
                        .voxel_at_indices(i, j, k)
                        .map_or_else(Voxel::empty, Voxel::new_from_type_without_flags);

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
                (Self::Empty, face_empty_counts)
            } else {
                // Most voxels in a uniform chunk are surrounded by neighbors,
                // so we assume this also holds for the boundary voxels for now
                // and update the boundary voxels later if the adjacent chunks
                // are not full
                first_voxel.add_flags(VoxelFlags::full_adjacency());

                (Self::Uniform(first_voxel), face_empty_counts)
            }
        } else {
            let face_distributions = face_empty_counts.to_face_distributions(CHUNK_SIZE_SQUARED);
            (
                Self::NonUniform {
                    start_voxel_idx,
                    face_distributions,
                },
                face_empty_counts,
            )
        }
    }

    const fn uniform(voxel: Voxel) -> Self {
        Self::Uniform(voxel)
    }

    const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    const fn start_voxel_idx_if_non_uniform(&self) -> Option<usize> {
        if let Self::NonUniform {
            start_voxel_idx, ..
        } = self
        {
            Some(*start_voxel_idx)
        } else {
            None
        }
    }

    const fn stored_voxel_count(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Uniform(_) => 1,
            Self::NonUniform { .. } => CHUNK_VOXEL_COUNT,
        }
    }

    fn update_internal_adjacencies(&self, voxels: &mut [Voxel]) {
        // We only need to update the internal adjacency if the chunk is
        // non-uniform
        let start_voxel_idx = if let Self::NonUniform {
            start_voxel_idx, ..
        } = self
        {
            *start_voxel_idx
        } else {
            return;
        };

        // Extract the sub-slice of voxels for this chunk so that we get
        // out-of-bounds when trying to access voxels outside the chunk
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
                    for (adjacent_indices, flag_for_current, flag_for_adjacent) in [
                        (
                            [i, j, k + 1],
                            VoxelFlags::HAS_ADJACENT_Z_UP,
                            VoxelFlags::HAS_ADJACENT_Z_DN,
                        ),
                        (
                            [i, j + 1, k],
                            VoxelFlags::HAS_ADJACENT_Y_UP,
                            VoxelFlags::HAS_ADJACENT_Y_DN,
                        ),
                        (
                            [i + 1, j, k],
                            VoxelFlags::HAS_ADJACENT_X_UP,
                            VoxelFlags::HAS_ADJACENT_X_DN,
                        ),
                    ] {
                        let adjacent_idx = linear_voxel_idx_within_chunk(&adjacent_indices);
                        match chunk_voxels.get_mut(adjacent_idx) {
                            Some(adjacent_voxel) if !adjacent_voxel.is_empty() => {
                                flags |= flag_for_current;
                                adjacent_voxel.add_flags(flag_for_adjacent);
                            }
                            _ => {}
                        }
                    }

                    chunk_voxels[idx].add_flags(flags);
                }
            }
        }
    }

    fn update_mutual_face_adjacencies(
        chunks: &mut [VoxelChunk],
        voxels: &mut Vec<Voxel>,
        lower_chunk_idx: ChunkIndex,
        upper_chunk_idx: ChunkIndex,
        dim: Dimension,
    ) {
        let lower_chunk = lower_chunk_idx.to_chunk(chunks);
        let upper_chunk = upper_chunk_idx.to_chunk(chunks);

        match (lower_chunk, upper_chunk) {
            // If both chunks are empty or uniform, there is nothing to do
            (Self::Empty, Self::Empty) | (Self::Uniform(_), Self::Uniform(_)) => {}
            // If one is uniform and the other is empty, we need to convert the
            // uniform chunk to non-uniform and clear its adjacencies to the
            // empty chunk
            (Self::Uniform(_), Self::Empty) => {
                let lower_chunk = &mut chunks[lower_chunk_idx.unwrap_idx()];
                lower_chunk.convert_to_non_uniform_if_uniform(voxels);
                Self::remove_all_outward_adjacencies_for_face(
                    voxels,
                    lower_chunk.start_voxel_idx_if_non_uniform().unwrap(),
                    Face::upper(dim),
                );
            }
            (Self::Empty, Self::Uniform(_)) => {
                let upper_chunk = &mut chunks[upper_chunk_idx.unwrap_idx()];
                upper_chunk.convert_to_non_uniform_if_uniform(voxels);
                Self::remove_all_outward_adjacencies_for_face(
                    voxels,
                    upper_chunk.start_voxel_idx_if_non_uniform().unwrap(),
                    Face::lower(dim),
                );
            }
            // If one is non-uniform and the other is empty, we need to clear
            // the adjacencies of the non-uniform chunk with the empty chunk
            (
                Self::NonUniform {
                    start_voxel_idx: lower_chunk_start_voxel_idx,
                    face_distributions: lower_chunk_face_distributions,
                },
                Self::Empty,
            ) => {
                // We can skip the update if there are no voxels on the face
                if lower_chunk_face_distributions[dim.idx()][1] != FaceVoxelDistribution::Empty {
                    Self::remove_all_outward_adjacencies_for_face(
                        voxels,
                        lower_chunk_start_voxel_idx,
                        Face::upper(dim),
                    );
                }
            }
            (
                Self::Empty,
                Self::NonUniform {
                    start_voxel_idx: upper_chunk_start_voxel_idx,
                    face_distributions: upper_chunk_face_distributions,
                },
            ) => {
                if upper_chunk_face_distributions[dim.idx()][0] != FaceVoxelDistribution::Empty {
                    Self::remove_all_outward_adjacencies_for_face(
                        voxels,
                        upper_chunk_start_voxel_idx,
                        Face::lower(dim),
                    );
                }
            }
            // If one is non-uniform and the other is uniform, we need to set
            // the adjacencies of the non-uniform chunk with the uniform chunk,
            // and if the adjoining face of the non-uniform chunk is not full,
            // we must convert the uniform chunk to non-uniform and update its
            // adjacencies as well
            (
                Self::NonUniform {
                    start_voxel_idx: lower_chunk_start_voxel_idx,
                    face_distributions: lower_chunk_face_distributions,
                    ..
                },
                Self::Uniform(_),
            ) => {
                let lower_chunk_face_distribution = lower_chunk_face_distributions[dim.idx()][1];

                if lower_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    Self::add_all_outward_adjacencies_for_face(
                        voxels,
                        lower_chunk_start_voxel_idx,
                        Face::upper(dim),
                    );
                }

                match lower_chunk_face_distribution {
                    FaceVoxelDistribution::Full => {}
                    FaceVoxelDistribution::Empty => {
                        let upper_chunk = &mut chunks[upper_chunk_idx.unwrap_idx()];
                        upper_chunk.convert_to_non_uniform_if_uniform(voxels);
                        Self::remove_all_outward_adjacencies_for_face(
                            voxels,
                            upper_chunk.start_voxel_idx_if_non_uniform().unwrap(),
                            Face::lower(dim),
                        );
                    }
                    FaceVoxelDistribution::Mixed => {
                        let upper_chunk = &mut chunks[upper_chunk_idx.unwrap_idx()];
                        upper_chunk.convert_to_non_uniform_if_uniform(voxels);
                        Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                            voxels,
                            upper_chunk.start_voxel_idx_if_non_uniform().unwrap(),
                            lower_chunk_start_voxel_idx,
                            Face::lower(dim),
                        );
                    }
                }
            }
            (
                Self::Uniform(_),
                Self::NonUniform {
                    start_voxel_idx: upper_chunk_start_voxel_idx,
                    face_distributions: upper_chunk_face_distributions,
                },
            ) => {
                let upper_chunk_face_distribution = upper_chunk_face_distributions[dim.idx()][0];

                if upper_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    Self::add_all_outward_adjacencies_for_face(
                        voxels,
                        upper_chunk_start_voxel_idx,
                        Face::lower(dim),
                    );
                }

                match upper_chunk_face_distribution {
                    FaceVoxelDistribution::Full => {}
                    FaceVoxelDistribution::Empty => {
                        let lower_chunk = &mut chunks[lower_chunk_idx.unwrap_idx()];
                        lower_chunk.convert_to_non_uniform_if_uniform(voxels);
                        Self::remove_all_outward_adjacencies_for_face(
                            voxels,
                            lower_chunk.start_voxel_idx_if_non_uniform().unwrap(),
                            Face::upper(dim),
                        );
                    }
                    FaceVoxelDistribution::Mixed => {
                        let lower_chunk = &mut chunks[lower_chunk_idx.unwrap_idx()];
                        lower_chunk.convert_to_non_uniform_if_uniform(voxels);
                        Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                            voxels,
                            lower_chunk.start_voxel_idx_if_non_uniform().unwrap(),
                            upper_chunk_start_voxel_idx,
                            Face::upper(dim),
                        );
                    }
                }
            }
            // If both chunks are non-uniform, we need to update the adjacencies
            // for both according to their adjoining faces
            (
                Self::NonUniform {
                    start_voxel_idx: lower_chunk_start_voxel_idx,
                    face_distributions: lower_chunk_face_distributions,
                },
                Self::NonUniform {
                    start_voxel_idx: upper_chunk_start_voxel_idx,
                    face_distributions: upper_chunk_face_distributions,
                },
            ) => {
                let lower_chunk_face_distribution = lower_chunk_face_distributions[dim.idx()][1];
                let upper_chunk_face_distribution = upper_chunk_face_distributions[dim.idx()][0];

                if lower_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    match upper_chunk_face_distribution {
                        FaceVoxelDistribution::Empty => {
                            Self::remove_all_outward_adjacencies_for_face(
                                voxels,
                                lower_chunk_start_voxel_idx,
                                Face::upper(dim),
                            );
                        }
                        FaceVoxelDistribution::Full => {
                            Self::add_all_outward_adjacencies_for_face(
                                voxels,
                                lower_chunk_start_voxel_idx,
                                Face::upper(dim),
                            );
                        }
                        FaceVoxelDistribution::Mixed => {
                            Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                                voxels,
                                lower_chunk_start_voxel_idx,
                                upper_chunk_start_voxel_idx,
                                Face::upper(dim),
                            );
                        }
                    }
                }

                if upper_chunk_face_distribution != FaceVoxelDistribution::Empty {
                    match lower_chunk_face_distribution {
                        FaceVoxelDistribution::Empty => {
                            Self::remove_all_outward_adjacencies_for_face(
                                voxels,
                                upper_chunk_start_voxel_idx,
                                Face::lower(dim),
                            );
                        }
                        FaceVoxelDistribution::Full => {
                            Self::add_all_outward_adjacencies_for_face(
                                voxels,
                                upper_chunk_start_voxel_idx,
                                Face::lower(dim),
                            );
                        }
                        FaceVoxelDistribution::Mixed => {
                            Self::update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
                                voxels,
                                upper_chunk_start_voxel_idx,
                                lower_chunk_start_voxel_idx,
                                Face::lower(dim),
                            );
                        }
                    }
                }
            }
        }
    }

    fn convert_to_non_uniform_if_uniform(&mut self, voxels: &mut Vec<Voxel>) {
        if let Self::Uniform(voxel) = self {
            let start_voxel_idx = voxels.len();
            voxels.reserve(CHUNK_VOXEL_COUNT);
            voxels.extend(iter::repeat(*voxel).take(CHUNK_VOXEL_COUNT));
            *self = Self::NonUniform {
                start_voxel_idx,
                face_distributions: [[FaceVoxelDistribution::Full; 2]; 3],
            };
        }
    }

    fn add_all_outward_adjacencies_for_face(
        voxels: &mut [Voxel],
        start_voxel_idx: usize,
        face: Face,
    ) {
        Self::update_all_outward_adjacencies_for_face(
            voxels,
            start_voxel_idx,
            face,
            &Voxel::add_flags,
        );
    }

    fn remove_all_outward_adjacencies_for_face(
        voxels: &mut [Voxel],
        start_voxel_idx: usize,
        face: Face,
    ) {
        Self::update_all_outward_adjacencies_for_face(
            voxels,
            start_voxel_idx,
            face,
            &Voxel::remove_flags,
        );
    }

    fn update_all_outward_adjacencies_for_face(
        voxels: &mut [Voxel],
        start_voxel_idx: usize,
        face: Face,
        update_flags: &impl Fn(&mut Voxel, VoxelFlags),
    ) {
        let chunk_voxels = &mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT];

        match face {
            Face::LowerX => {
                for j in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        let idx = linear_voxel_idx_within_chunk(&[0, j, k]);
                        update_flags(&mut chunk_voxels[idx], VoxelFlags::HAS_ADJACENT_X_DN);
                    }
                }
            }
            Face::UpperX => {
                for j in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        let idx = linear_voxel_idx_within_chunk(&[CHUNK_SIZE - 1, j, k]);
                        update_flags(&mut chunk_voxels[idx], VoxelFlags::HAS_ADJACENT_X_UP);
                    }
                }
            }
            Face::LowerY => {
                for i in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        let idx = linear_voxel_idx_within_chunk(&[i, 0, k]);
                        update_flags(&mut chunk_voxels[idx], VoxelFlags::HAS_ADJACENT_Y_DN);
                    }
                }
            }
            Face::UpperY => {
                for i in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        let idx = linear_voxel_idx_within_chunk(&[i, CHUNK_SIZE - 1, k]);
                        update_flags(&mut chunk_voxels[idx], VoxelFlags::HAS_ADJACENT_Y_UP);
                    }
                }
            }
            Face::LowerZ => {
                for i in 0..CHUNK_SIZE {
                    for j in 0..CHUNK_SIZE {
                        let idx = linear_voxel_idx_within_chunk(&[i, j, 0]);
                        update_flags(&mut chunk_voxels[idx], VoxelFlags::HAS_ADJACENT_Z_DN);
                    }
                }
            }
            Face::UpperZ => {
                for i in 0..CHUNK_SIZE {
                    for j in 0..CHUNK_SIZE {
                        let idx = linear_voxel_idx_within_chunk(&[i, j, CHUNK_SIZE - 1]);
                        update_flags(&mut chunk_voxels[idx], VoxelFlags::HAS_ADJACENT_Z_UP);
                    }
                }
            }
        }
    }

    fn update_outward_adjacencies_with_non_uniform_adjacent_chunk_for_face(
        voxels: &mut [Voxel],
        current_chunk_start_voxel_idx: usize,
        adjacent_chunk_start_voxel_idx: usize,
        face: Face,
    ) {
        let (current_chunk_voxels, adjacent_chunk_voxels) = extract_slice_segments_mut(
            voxels,
            current_chunk_start_voxel_idx,
            adjacent_chunk_start_voxel_idx,
            CHUNK_VOXEL_COUNT,
        );

        let mut update_adjacency =
            |current_indices: &[usize; 3], adjacent_indices: &[usize; 3], flag: VoxelFlags| {
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
            };

        match face {
            Face::LowerX => {
                for j in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        update_adjacency(
                            &[0, j, k],
                            &[CHUNK_SIZE - 1, j, k],
                            VoxelFlags::HAS_ADJACENT_X_DN,
                        );
                    }
                }
            }
            Face::UpperX => {
                for j in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        update_adjacency(
                            &[CHUNK_SIZE - 1, j, k],
                            &[0, j, k],
                            VoxelFlags::HAS_ADJACENT_X_UP,
                        );
                    }
                }
            }
            Face::LowerY => {
                for i in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        update_adjacency(
                            &[i, 0, k],
                            &[i, CHUNK_SIZE - 1, k],
                            VoxelFlags::HAS_ADJACENT_Y_DN,
                        );
                    }
                }
            }
            Face::UpperY => {
                for i in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        update_adjacency(
                            &[i, CHUNK_SIZE - 1, k],
                            &[i, 0, k],
                            VoxelFlags::HAS_ADJACENT_Y_UP,
                        );
                    }
                }
            }
            Face::LowerZ => {
                for i in 0..CHUNK_SIZE {
                    for j in 0..CHUNK_SIZE {
                        update_adjacency(
                            &[i, j, 0],
                            &[i, j, CHUNK_SIZE - 1],
                            VoxelFlags::HAS_ADJACENT_Z_DN,
                        );
                    }
                }
            }
            Face::UpperZ => {
                for i in 0..CHUNK_SIZE {
                    for j in 0..CHUNK_SIZE {
                        update_adjacency(
                            &[i, j, CHUNK_SIZE - 1],
                            &[i, j, 0],
                            VoxelFlags::HAS_ADJACENT_Z_UP,
                        );
                    }
                }
            }
        }
    }
}

impl FaceEmptyCounts {
    const fn zero() -> Self {
        Self([[0; 2]; 3])
    }

    const fn same(count: usize) -> Self {
        Self([[count; 2]; 3])
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

    fn add_x_dn(&mut self, other: &Self) {
        self.0[0][0] += other.0[0][0];
    }
    fn add_x_up(&mut self, other: &Self) {
        self.0[0][1] += other.0[0][1];
    }
    fn add_y_dn(&mut self, other: &Self) {
        self.0[1][0] += other.0[1][0];
    }
    fn add_y_up(&mut self, other: &Self) {
        self.0[1][1] += other.0[1][1];
    }
    fn add_z_dn(&mut self, other: &Self) {
        self.0[2][0] += other.0[2][0];
    }
    fn add_z_up(&mut self, other: &Self) {
        self.0[2][1] += other.0[2][1];
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

impl Voxel {
    /// Creates a new voxel with the given property ID and state flags.
    const fn new(property_id: PropertyID, flags: VoxelFlags) -> Self {
        Self { property_id, flags }
    }

    /// Creates a new voxel with the given property ID and no set state flags.
    const fn new_without_flags(property_id: PropertyID) -> Self {
        Self::new(property_id, VoxelFlags::empty())
    }

    /// Creates a new voxel with the given `VoxelType` and no set state flags.
    const fn new_from_type_without_flags(voxel_type: VoxelType) -> Self {
        Self::new_without_flags(PropertyID::from_voxel_type(voxel_type))
    }

    /// Creates a new empty voxel.
    const fn empty() -> Self {
        Self {
            property_id: PropertyID::dummy(),
            flags: VoxelFlags::IS_EMPTY,
        }
    }

    /// Whether the voxel is empty.
    pub fn is_empty(&self) -> bool {
        self.flags.contains(VoxelFlags::IS_EMPTY)
    }

    /// Returns the flags encoding the state of the voxel.
    pub fn flags(&self) -> VoxelFlags {
        self.flags
    }

    /// Sets the given state flags for the voxel (this will not clear any
    /// existing flags).
    fn add_flags(&mut self, flags: VoxelFlags) {
        self.flags.insert(flags);
    }

    /// Unsets the given state flags for the voxel.
    fn remove_flags(&mut self, flags: VoxelFlags) {
        self.flags.remove(flags);
    }
}

impl PropertyID {
    /// Creates a new property ID for the given `VoxelType`.
    pub const fn from_voxel_type(voxel_type: VoxelType) -> Self {
        Self(voxel_type as u8)
    }

    const fn dummy() -> Self {
        Self(u8::MAX)
    }
}

impl VoxelFlags {
    const fn full_adjacency() -> Self {
        Self::HAS_ADJACENT_X_DN
            .union(Self::HAS_ADJACENT_X_UP)
            .union(Self::HAS_ADJACENT_Y_DN)
            .union(Self::HAS_ADJACENT_Y_UP)
            .union(Self::HAS_ADJACENT_Z_DN)
            .union(Self::HAS_ADJACENT_Z_UP)
    }
}

impl Dimension {
    const fn idx(self) -> usize {
        self as usize
    }
}

impl Face {
    const fn lower(dim: Dimension) -> Self {
        match dim {
            Dimension::X => Self::LowerX,
            Dimension::Y => Self::LowerY,
            Dimension::Z => Self::LowerZ,
        }
    }

    const fn upper(dim: Dimension) -> Self {
        match dim {
            Dimension::X => Self::UpperX,
            Dimension::Y => Self::UpperY,
            Dimension::Z => Self::UpperZ,
        }
    }
}

impl ChunkIndex {
    fn to_chunk(self, chunks: &[VoxelChunk]) -> VoxelChunk {
        match self {
            Self::AbsentEmpty => VoxelChunk::Empty,
            Self::AbsentUniform(voxel) => VoxelChunk::uniform(voxel),
            Self::Present(idx) => chunks[idx].clone(),
        }
    }

    fn to_superchunk(self, superchunks: &[VoxelSuperchunk]) -> VoxelSuperchunk {
        match self {
            Self::AbsentEmpty => VoxelSuperchunk::Empty,
            Self::AbsentUniform(voxel) => VoxelSuperchunk::uniform(voxel),
            Self::Present(idx) => superchunks[idx].clone(),
        }
    }

    fn map_idx(&self, f: impl FnOnce(usize) -> usize) -> Self {
        match self {
            Self::AbsentEmpty => Self::AbsentEmpty,
            Self::AbsentUniform(voxel) => Self::AbsentUniform(*voxel),
            Self::Present(idx) => Self::Present(f(*idx)),
        }
    }

    fn unwrap_idx(&self) -> usize {
        match self {
            Self::Present(idx) => *idx,
            _ => panic!("Tried to unwrap absent chunk index"),
        }
    }
}

/// Computes the index into a superchunk's flattened chunk grid of the chunk
/// containing the voxel at the given indices in the parent object's voxel grid.
const fn linear_chunk_idx_within_superchunk_from_object_voxel_indices(
    i: usize,
    j: usize,
    k: usize,
) -> usize {
    let chunk_indices = chunk_indices_within_superchunk_from_object_voxel_indices(i, j, k);
    linear_chunk_idx_within_superchunk(&chunk_indices)
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

/// Computes the index into a superchunk's flattened chunk grid of the chunk
/// with the given 3D index in the chunk grid.
const fn linear_chunk_idx_within_superchunk(chunk_indices: &[usize; 3]) -> usize {
    chunk_indices[0] * SUPERCHUNK_SIZE_SQUARED
        + chunk_indices[1] * SUPERCHUNK_SIZE
        + chunk_indices[2]
}

/// Computes the index into a chunk's flattened voxel grid of the voxel with the
/// given 3D index in the voxel grid.
const fn linear_voxel_idx_within_chunk(voxel_indices: &[usize; 3]) -> usize {
    voxel_indices[0] * CHUNK_SIZE_SQUARED + voxel_indices[1] * CHUNK_SIZE + voxel_indices[2]
}

/// Computes the 3D index in the parent object's superchunk grid of the
/// superchunk containing the voxel at the given indices in the object's voxel
/// grid.
///
/// Since chunks and superchunks have a power-of-two number of voxels along each
/// axis, the superchunk index is encoded in the upper bits of the corresponding
/// object voxel index.
const fn superchunk_indices_from_object_voxel_indices(i: usize, j: usize, k: usize) -> [usize; 3] {
    [
        i >> SUPERCHUNK_IDX_SHIFT,
        j >> SUPERCHUNK_IDX_SHIFT,
        k >> SUPERCHUNK_IDX_SHIFT,
    ]
}

/// Computes the 3D index in a superchunk's chunk grid of the chunk containing
/// the voxel at the given indices in the parent object's voxel grid.
///
/// Since chunks and superchunks have a power-of-two number of voxels along each
/// axis, the chunk index is encoded in the middle bits of the corresponding
/// object voxel index.
const fn chunk_indices_within_superchunk_from_object_voxel_indices(
    i: usize,
    j: usize,
    k: usize,
) -> [usize; 3] {
    [
        (i >> CHUNK_IDX_SHIFT) & CHUNK_IDX_MASK,
        (j >> CHUNK_IDX_SHIFT) & CHUNK_IDX_MASK,
        (k >> CHUNK_IDX_SHIFT) & CHUNK_IDX_MASK,
    ]
}

/// Computes the 3D index in a chunk's voxel grid of the voxel at the given
/// indices in the parent object's voxel grid.
///
/// Since chunks and superchunks have a power-of-two number of voxels along each
/// axis, the voxel index within the chunk is encoded in the lower bits of the
/// corresponding object voxel index.
const fn voxel_indices_within_chunk_from_object_voxel_indices(
    i: usize,
    j: usize,
    k: usize,
) -> [usize; 3] {
    [i & VOXEL_IDX_MASK, j & VOXEL_IDX_MASK, k & VOXEL_IDX_MASK]
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::voxel::generation::UniformBoxVoxelGenerator;

    struct BoxVoxelGenerator {
        shape: [usize; 3],
        offset: [usize; 3],
        voxel_type: Option<VoxelType>,
    }

    struct ManualVoxelGenerator<const N: usize> {
        voxels: [[[u8; N]; N]; N],
        offset: [usize; 3],
    }

    impl BoxVoxelGenerator {
        fn new(shape: [usize; 3], offset: [usize; 3], voxel_type: Option<VoxelType>) -> Self {
            Self {
                shape,
                offset,
                voxel_type,
            }
        }

        fn empty(shape: [usize; 3]) -> Self {
            Self::new(shape, [0; 3], None)
        }

        fn single(voxel_type: Option<VoxelType>) -> Self {
            Self::new([1, 1, 1], [0; 3], voxel_type)
        }

        fn single_default() -> Self {
            Self::single(Some(VoxelType::Default))
        }

        fn with_default(shape: [usize; 3]) -> Self {
            Self::offset_with_default(shape, [0; 3])
        }

        fn offset_with_default(shape: [usize; 3], offset: [usize; 3]) -> Self {
            Self::new(shape, offset, Some(VoxelType::Default))
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

    impl VoxelGenerator<f64> for BoxVoxelGenerator {
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

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
            if i >= self.offset[0]
                && i < self.offset[0] + self.shape[0]
                && j >= self.offset[1]
                && j < self.offset[1] + self.shape[1]
                && k >= self.offset[2]
                && k < self.offset[2] + self.shape[2]
            {
                self.voxel_type
            } else {
                None
            }
        }
    }

    impl<const N: usize> VoxelGenerator<f64> for ManualVoxelGenerator<N> {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            [self.offset[0] + N, self.offset[1] + N, self.offset[2] + N]
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
            if i >= self.offset[0]
                && i < self.offset[0] + N
                && j >= self.offset[1]
                && j < self.offset[1] + N
                && k >= self.offset[2]
                && k < self.offset[2] + N
                && self.voxels[i - self.offset[0]][j - self.offset[1]][k - self.offset[2]] != 0
            {
                Some(VoxelType::Default)
            } else {
                None
            }
        }
    }

    #[test]
    fn should_yield_none_when_generating_object_with_empty_grid() {
        assert!(ChunkedVoxelObject::generate(&BoxVoxelGenerator::with_default([0; 3])).is_none());
    }

    #[test]
    fn should_yield_none_when_generating_object_of_empty_voxels() {
        assert!(ChunkedVoxelObject::generate(&BoxVoxelGenerator::single(None)).is_none());
        assert!(ChunkedVoxelObject::generate(&BoxVoxelGenerator::empty([2, 3, 4])).is_none());
    }

    #[test]
    fn should_generate_object_with_single_voxel() {
        let generator = BoxVoxelGenerator::single_default();
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        assert_eq!(object.voxel_extent(), generator.voxel_extent());
        assert_eq!(object.n_superchunks_per_axis(), 1);
        assert_eq!(object.full_grid_size(), SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.occupied_range(0), 0..CHUNK_SIZE);
        assert_eq!(object.occupied_range(1), 0..CHUNK_SIZE);
        assert_eq!(object.occupied_range(2), 0..CHUNK_SIZE);
        assert_eq!(object.stored_voxel_count(), CHUNK_VOXEL_COUNT);
    }

    #[test]
    fn should_generate_object_with_single_uniform_superchunk() {
        let generator = BoxVoxelGenerator::with_default([SUPERCHUNK_SIZE_IN_VOXELS; 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        assert_eq!(object.n_superchunks_per_axis(), 1);
        assert_eq!(object.full_grid_size(), SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.occupied_range(0), 0..SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.occupied_range(1), 0..SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.occupied_range(2), 0..SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.stored_voxel_count(), 1);
    }

    #[test]
    fn should_generate_object_with_single_uniform_superchunk_plus_one_voxel() {
        let generator = BoxVoxelGenerator::with_default([SUPERCHUNK_SIZE_IN_VOXELS + 1; 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        assert_eq!(object.n_superchunks_per_axis(), 2);
        assert_eq!(object.full_grid_size(), 2 * SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(
            object.occupied_range(0),
            0..SUPERCHUNK_SIZE_IN_VOXELS + CHUNK_SIZE
        );
        assert_eq!(
            object.occupied_range(1),
            0..SUPERCHUNK_SIZE_IN_VOXELS + CHUNK_SIZE
        );
        assert_eq!(
            object.occupied_range(2),
            0..SUPERCHUNK_SIZE_IN_VOXELS + CHUNK_SIZE
        );
        assert_eq!(
            object.stored_voxel_count(),
            // First superchunk (full) + faces of the three adjacent superchunks
            // + edges of the three semi-diagonal superchunks + corner of the
            //   fully diagonal superchunk
            1 + 3 * CHUNK_VOXEL_COUNT * SUPERCHUNK_SIZE.pow(2)
                + 3 * CHUNK_VOXEL_COUNT * SUPERCHUNK_SIZE
                + CHUNK_VOXEL_COUNT
        );
    }

    #[test]
    fn should_generate_object_with_single_uniform_chunk() {
        let generator = BoxVoxelGenerator::with_default([CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        assert_eq!(object.n_superchunks_per_axis(), 1);
        assert_eq!(object.full_grid_size(), SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.occupied_range(0), 0..CHUNK_SIZE);
        assert_eq!(object.occupied_range(1), 0..CHUNK_SIZE);
        assert_eq!(object.occupied_range(2), 0..CHUNK_SIZE);
        assert_eq!(object.stored_voxel_count(), 1);
    }

    #[test]
    fn should_generate_object_with_single_offset_uniform_chunk() {
        let generator = BoxVoxelGenerator::offset_with_default([CHUNK_SIZE; 3], [CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        assert_eq!(object.n_superchunks_per_axis(), 1);
        assert_eq!(object.full_grid_size(), SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.occupied_range(0), CHUNK_SIZE..2 * CHUNK_SIZE);
        assert_eq!(object.occupied_range(1), CHUNK_SIZE..2 * CHUNK_SIZE);
        assert_eq!(object.occupied_range(2), CHUNK_SIZE..2 * CHUNK_SIZE);
        assert_eq!(object.stored_voxel_count(), 1);
    }

    #[test]
    fn should_get_correct_voxels_in_small_grid() {
        let generator = ManualVoxelGenerator::<3>::new([
            [[1, 1, 0], [1, 0, 1], [0, 1, 0]],
            [[0, 1, 1], [1, 0, 0], [1, 0, 1]],
            [[1, 1, 0], [1, 1, 1], [0, 0, 0]],
        ]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
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
        let offset = [SUPERCHUNK_SIZE_IN_VOXELS - 2; 3];
        let generator = ManualVoxelGenerator::<3>::with_offset(
            [
                [[1, 1, 0], [1, 0, 1], [0, 1, 0]],
                [[0, 1, 1], [1, 0, 0], [1, 0, 1]],
                [[1, 1, 0], [1, 1, 1], [0, 0, 0]],
            ],
            offset,
        );
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
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

        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();

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

        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();

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

        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();

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
        let generator = UniformBoxVoxelGenerator::new(VoxelType::Default, 0.25_f32, 1, 1, 1);
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();
    }

    #[test]
    fn should_compute_correct_adjacencies_for_single_chunk() {
        let generator = UniformBoxVoxelGenerator::new(
            VoxelType::Default,
            0.25_f32,
            CHUNK_SIZE,
            CHUNK_SIZE,
            CHUNK_SIZE,
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();
    }

    #[test]
    fn should_compute_correct_adjacencies_for_barely_two_chunks() {
        let generator = UniformBoxVoxelGenerator::new(
            VoxelType::Default,
            0.25_f32,
            CHUNK_SIZE + 1,
            CHUNK_SIZE,
            CHUNK_SIZE,
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();

        let generator = UniformBoxVoxelGenerator::new(
            VoxelType::Default,
            0.25_f32,
            CHUNK_SIZE,
            CHUNK_SIZE + 1,
            CHUNK_SIZE,
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();

        let generator = UniformBoxVoxelGenerator::new(
            VoxelType::Default,
            0.25_f32,
            CHUNK_SIZE,
            CHUNK_SIZE,
            CHUNK_SIZE + 1,
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();
    }

    #[test]
    fn should_compute_correct_adjacencies_with_column_taking_barely_two_superchunks() {
        let generator = UniformBoxVoxelGenerator::new(
            VoxelType::Default,
            0.25_f32,
            SUPERCHUNK_SIZE_IN_VOXELS + 1,
            1,
            1,
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();

        let generator = UniformBoxVoxelGenerator::new(
            VoxelType::Default,
            0.25_f32,
            1,
            SUPERCHUNK_SIZE_IN_VOXELS + 1,
            1,
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();

        let generator = UniformBoxVoxelGenerator::new(
            VoxelType::Default,
            0.25_f32,
            1,
            1,
            SUPERCHUNK_SIZE_IN_VOXELS + 1,
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.update_adjacencies();
        object.validate_adjacencies();
    }
}
