//!

use crate::{
    num::Float,
    voxel::{VoxelGenerator, VoxelType},
};
use bitflags::bitflags;
use std::{iter, ops::Range};

#[derive(Clone, Debug)]
pub struct ChunkedVoxelObject {
    voxel_extent: f64,
    n_superchunks_per_axis: usize,
    occupied_chunks: [Range<usize>; 3],
    superchunks: Vec<VoxelSuperchunk>,
    chunks: Vec<VoxelChunk>,
    voxels: Vec<Voxel>,
}

#[derive(Clone, Debug)]
pub struct VoxelSuperchunk {
    chunks: SuperchunkChunks,
}

#[derive(Clone, Debug)]
enum SuperchunkChunks {
    None,
    Same(Voxel),
    Different { start_chunk_idx: usize },
}

#[derive(Clone, Debug)]
pub struct VoxelChunk {
    voxels: ChunkVoxels,
}

#[derive(Clone, Debug)]
enum ChunkVoxels {
    None,
    Same(Voxel),
    Different { start_voxel_idx: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voxel {
    property_id: PropertyID,
    flags: VoxelFlags,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropertyID(u8);

bitflags! {
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct VoxelFlags: u8 {
        const IS_EMPTY          = 1 << 0;
        const HAS_ADJACENT_X_DN = 1 << 2;
        const HAS_ADJACENT_Y_DN = 1 << 3;
        const HAS_ADJACENT_Z_DN = 1 << 4;
        const HAS_ADJACENT_X_UP = 1 << 5;
        const HAS_ADJACENT_Y_UP = 1 << 6;
        const HAS_ADJACENT_Z_UP = 1 << 7;
    }
}

const LOG2_CHUNK_SIZE: usize = 4;
const CHUNK_SIZE: usize = 1 << LOG2_CHUNK_SIZE;
const CHUNK_VOXEL_COUNT: usize = CHUNK_SIZE.pow(3);

const LOG2_SUPERCHUNK_SIZE: usize = 3;
const SUPERCHUNK_SIZE: usize = 1 << LOG2_SUPERCHUNK_SIZE;
const SUPERCHUNK_SIZE_IN_VOXELS: usize = SUPERCHUNK_SIZE * CHUNK_SIZE;
const SUPERCHUNK_CHUNK_COUNT: usize = SUPERCHUNK_SIZE.pow(3);

const SUPERCHUNK_IDX_SHIFT: usize = LOG2_CHUNK_SIZE + LOG2_SUPERCHUNK_SIZE;
const CHUNK_IDX_SHIFT: usize = LOG2_CHUNK_SIZE;
const CHUNK_IDX_MASK: usize = (1 << LOG2_SUPERCHUNK_SIZE) - 1;
const VOXEL_IDX_MASK: usize = (1 << LOG2_CHUNK_SIZE) - 1;

#[allow(clippy::reversed_empty_ranges)]
const REVERSED_MAX_RANGE: Range<usize> = usize::MAX..usize::MIN;

impl ChunkedVoxelObject {
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
                        generator,
                        [superchunk_i, superchunk_j, superchunk_k],
                        &mut chunks,
                        &mut voxels,
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

    pub fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    pub fn n_superchunks_per_axis(&self) -> usize {
        self.n_superchunks_per_axis
    }

    pub fn full_grid_size(&self) -> usize {
        self.n_superchunks_per_axis * SUPERCHUNK_SIZE_IN_VOXELS
    }

    pub fn occupied_chunks(&self) -> &[Range<usize>; 3] {
        &self.occupied_chunks
    }

    pub fn occupied_range(&self, axis: usize) -> Range<usize> {
        self.occupied_chunks[axis].start * CHUNK_SIZE..self.occupied_chunks[axis].end * CHUNK_SIZE
    }

    pub fn stored_voxel_count(&self) -> usize {
        self.superchunks
            .iter()
            .map(|superchunk| superchunk.stored_voxel_count(&self.chunks))
            .sum()
    }

    pub fn get_voxel(&self, i: usize, j: usize, k: usize) -> Option<&Voxel> {
        let superchunk_idx = self.linear_superchunk_idx_from_global_voxel_indices(i, j, k);
        let superchunk = self.superchunks.get(superchunk_idx)?;
        match &superchunk.chunks {
            SuperchunkChunks::None => None,
            SuperchunkChunks::Same(voxel) => Some(voxel),
            SuperchunkChunks::Different { start_chunk_idx } => {
                let chunk_idx = start_chunk_idx
                    + Self::linear_chunk_idx_within_superchunk_from_global_voxel_indices(i, j, k);
                let chunk = &self.chunks[chunk_idx];
                match &chunk.voxels {
                    ChunkVoxels::None => None,
                    ChunkVoxels::Same(voxel) => Some(voxel),
                    ChunkVoxels::Different { start_voxel_idx } => {
                        let voxel_idx = start_voxel_idx
                            + Self::linear_voxel_idx_within_chunk_from_global_voxel_indices(
                                i, j, k,
                            );
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

    pub fn update_adjacency(&mut self) {
        for start_voxel_idx in self
            .chunks
            .iter()
            .filter_map(VoxelChunk::start_voxel_idx_if_different)
        {
            Self::update_internal_adjacency_in_chunk(start_voxel_idx, &mut self.voxels);
        }
    }

    fn update_internal_adjacency_in_chunk(start_voxel_idx: usize, voxels: &mut [Voxel]) {
        // Extract the sub-slice of voxels for this chunk so that we get
        // out-of-bounds when trying to access voxels outside the chunk
        let chunk_voxels = &mut voxels[start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT];

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let idx = Self::linear_voxel_idx_within_chunk(&[i, j, k]);

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
                            [i + 1, j, k],
                            VoxelFlags::HAS_ADJACENT_X_UP,
                            VoxelFlags::HAS_ADJACENT_X_DN,
                        ),
                        (
                            [i, j + 1, k],
                            VoxelFlags::HAS_ADJACENT_Y_UP,
                            VoxelFlags::HAS_ADJACENT_Y_DN,
                        ),
                        (
                            [i, j, k + 1],
                            VoxelFlags::HAS_ADJACENT_Z_UP,
                            VoxelFlags::HAS_ADJACENT_Z_DN,
                        ),
                    ] {
                        let adjacent_idx = Self::linear_voxel_idx_within_chunk(&adjacent_indices);
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

    fn linear_superchunk_idx_from_global_voxel_indices(
        &self,
        i: usize,
        j: usize,
        k: usize,
    ) -> usize {
        let superchunk_indices = superchunk_indices_from_global_voxel_indices(i, j, k);
        self.linear_superchunk_idx(&superchunk_indices)
    }

    const fn linear_chunk_idx_within_superchunk_from_global_voxel_indices(
        i: usize,
        j: usize,
        k: usize,
    ) -> usize {
        let chunk_indices = chunk_indices_within_superchunk_from_global_voxel_indices(i, j, k);
        Self::linear_chunk_idx_within_superchunk(&chunk_indices)
    }

    const fn linear_voxel_idx_within_chunk_from_global_voxel_indices(
        i: usize,
        j: usize,
        k: usize,
    ) -> usize {
        let voxel_indices = voxel_indices_within_chunk_from_global_voxel_indices(i, j, k);
        Self::linear_voxel_idx_within_chunk(&voxel_indices)
    }

    fn linear_superchunk_idx(&self, superchunk_indices: &[usize; 3]) -> usize {
        superchunk_indices[0] * self.n_superchunks_per_axis * self.n_superchunks_per_axis
            + superchunk_indices[1] * self.n_superchunks_per_axis
            + superchunk_indices[2]
    }

    const fn linear_chunk_idx_within_superchunk(chunk_indices: &[usize; 3]) -> usize {
        chunk_indices[0] * SUPERCHUNK_SIZE * SUPERCHUNK_SIZE
            + chunk_indices[1] * SUPERCHUNK_SIZE
            + chunk_indices[2]
    }

    const fn linear_voxel_idx_within_chunk(voxel_indices: &[usize; 3]) -> usize {
        voxel_indices[0] * CHUNK_SIZE * CHUNK_SIZE
            + voxel_indices[1] * CHUNK_SIZE
            + voxel_indices[2]
    }
}

impl VoxelSuperchunk {
    fn generate<G, F>(
        generator: &G,
        superchunk_indices: [usize; 3],
        chunks: &mut Vec<VoxelChunk>,
        voxels: &mut Vec<Voxel>,
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

        let mut occupied_chunks_i = REVERSED_MAX_RANGE;
        let mut occupied_chunks_j = REVERSED_MAX_RANGE;
        let mut occupied_chunks_k = REVERSED_MAX_RANGE;

        for chunk_i in start_chunk_indices[0]..start_chunk_indices[0] + SUPERCHUNK_SIZE {
            for chunk_j in start_chunk_indices[1]..start_chunk_indices[1] + SUPERCHUNK_SIZE {
                for chunk_k in start_chunk_indices[2]..start_chunk_indices[2] + SUPERCHUNK_SIZE {
                    let chunk =
                        VoxelChunk::generate(generator, [chunk_i, chunk_j, chunk_k], voxels);

                    if is_uniform {
                        match (&first_voxel, &chunk.voxels) {
                            (Some(first_voxel), ChunkVoxels::None) => {
                                is_uniform = first_voxel.is_empty();
                            }
                            (Some(first_voxel), ChunkVoxels::Same(voxel)) => {
                                is_uniform = first_voxel == voxel;
                            }
                            (_, ChunkVoxels::Different { .. }) => {
                                is_uniform = false;
                            }
                            (None, ChunkVoxels::None) => {
                                first_voxel = Some(Voxel::empty());
                            }
                            (None, ChunkVoxels::Same(voxel)) => {
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

                    chunks.push(chunk);
                }
            }
        }

        let occupied_chunks = [occupied_chunks_i, occupied_chunks_j, occupied_chunks_k];

        if is_uniform {
            chunks.truncate(start_chunk_idx);
            let first_voxel = first_voxel.unwrap();
            (
                Self {
                    chunks: if first_voxel.is_empty() {
                        SuperchunkChunks::None
                    } else {
                        SuperchunkChunks::Same(first_voxel)
                    },
                },
                occupied_chunks,
            )
        } else {
            (
                Self {
                    chunks: SuperchunkChunks::Different { start_chunk_idx },
                },
                occupied_chunks,
            )
        }
    }

    fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    fn stored_voxel_count(&self, chunks: &[VoxelChunk]) -> usize {
        match &self.chunks {
            SuperchunkChunks::None => 0,
            SuperchunkChunks::Same(_) => 1,
            &SuperchunkChunks::Different { start_chunk_idx } => chunks
                [start_chunk_idx..start_chunk_idx + SUPERCHUNK_CHUNK_COUNT]
                .iter()
                .map(VoxelChunk::stored_voxel_count)
                .sum(),
        }
    }

    fn expand_same(&mut self, chunks: &mut Vec<VoxelChunk>) {
        if let &SuperchunkChunks::Same(voxel) = &self.chunks {
            let start_chunk_idx = chunks.len();
            chunks.reserve(SUPERCHUNK_CHUNK_COUNT);
            chunks.extend(iter::repeat(VoxelChunk::same(voxel)).take(SUPERCHUNK_CHUNK_COUNT));
            self.chunks = SuperchunkChunks::Different { start_chunk_idx };
        }
    }
}

impl SuperchunkChunks {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl VoxelChunk {
    fn generate<G, F>(
        generator: &G,
        global_chunk_indices: [usize; 3],
        voxels: &mut Vec<Voxel>,
    ) -> Self
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
            return Self {
                voxels: ChunkVoxels::None,
            };
        }

        let first_voxel = generator
            .voxel_at_indices(origin[0], origin[1], origin[2])
            .map_or_else(Voxel::empty, Voxel::new_from_type_without_flags);
        let mut is_uniform = true;

        let start_voxel_idx = voxels.len();
        voxels.reserve(CHUNK_VOXEL_COUNT);

        for i in origin[0]..origin[0] + CHUNK_SIZE {
            for j in origin[1]..origin[1] + CHUNK_SIZE {
                for k in origin[2]..origin[2] + CHUNK_SIZE {
                    let voxel = generator
                        .voxel_at_indices(i, j, k)
                        .map_or_else(Voxel::empty, Voxel::new_from_type_without_flags);

                    if is_uniform && voxel != first_voxel {
                        is_uniform = false;
                    }

                    voxels.push(voxel);
                }
            }
        }

        if is_uniform {
            voxels.truncate(start_voxel_idx);
            Self {
                voxels: if first_voxel.is_empty() {
                    ChunkVoxels::None
                } else {
                    ChunkVoxels::Same(first_voxel)
                },
            }
        } else {
            Self {
                voxels: ChunkVoxels::Different { start_voxel_idx },
            }
        }
    }

    fn same(voxel: Voxel) -> Self {
        Self {
            voxels: ChunkVoxels::Same(voxel),
        }
    }

    fn start_voxel_idx_if_different(&self) -> Option<usize> {
        if let ChunkVoxels::Different { start_voxel_idx } = &self.voxels {
            Some(*start_voxel_idx)
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.voxels.is_empty()
    }

    fn stored_voxel_count(&self) -> usize {
        match &self.voxels {
            ChunkVoxels::None => 0,
            ChunkVoxels::Same(_) => 1,
            ChunkVoxels::Different { .. } => CHUNK_VOXEL_COUNT,
        }
    }

    fn expand_same(&mut self, voxels: &mut Vec<Voxel>) {
        if let &ChunkVoxels::Same(voxel) = &self.voxels {
            let start_voxel_idx = voxels.len();
            voxels.reserve(CHUNK_VOXEL_COUNT);
            voxels.extend(iter::repeat(voxel).take(CHUNK_VOXEL_COUNT));
            self.voxels = ChunkVoxels::Different { start_voxel_idx };
        }
    }
}

impl ChunkVoxels {
    fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Voxel {
    pub const fn new(property_id: PropertyID, flags: VoxelFlags) -> Self {
        Self { property_id, flags }
    }

    pub const fn new_without_flags(property_id: PropertyID) -> Self {
        Self::new(property_id, VoxelFlags::empty())
    }

    pub const fn new_from_type_without_flags(voxel_type: VoxelType) -> Self {
        Self::new_without_flags(PropertyID::from_voxel_type(voxel_type))
    }

    pub const fn empty() -> Self {
        Self {
            property_id: PropertyID::dummy(),
            flags: VoxelFlags::IS_EMPTY,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.flags.contains(VoxelFlags::IS_EMPTY)
    }

    pub fn flags(&self) -> VoxelFlags {
        self.flags
    }

    pub fn add_flags(&mut self, flags: VoxelFlags) {
        self.flags |= flags;
    }
}

impl PropertyID {
    pub const fn from_voxel_type(voxel_type: VoxelType) -> Self {
        Self(voxel_type as u8)
    }

    const fn dummy() -> Self {
        Self(u8::MAX)
    }
}

const fn superchunk_indices_from_global_voxel_indices(i: usize, j: usize, k: usize) -> [usize; 3] {
    [
        i >> SUPERCHUNK_IDX_SHIFT,
        j >> SUPERCHUNK_IDX_SHIFT,
        k >> SUPERCHUNK_IDX_SHIFT,
    ]
}

const fn chunk_indices_within_superchunk_from_global_voxel_indices(
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

const fn voxel_indices_within_chunk_from_global_voxel_indices(
    i: usize,
    j: usize,
    k: usize,
) -> [usize; 3] {
    [i & VOXEL_IDX_MASK, j & VOXEL_IDX_MASK, k & VOXEL_IDX_MASK]
}

#[cfg(test)]
mod test {
    use super::*;

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
        object.update_adjacency();

        assert_eq!(
            object.get_voxel(1, 1, 1).unwrap().flags(),
            VoxelFlags::HAS_ADJACENT_X_DN
                | VoxelFlags::HAS_ADJACENT_X_UP
                | VoxelFlags::HAS_ADJACENT_Y_DN
                | VoxelFlags::HAS_ADJACENT_Y_UP
                | VoxelFlags::HAS_ADJACENT_Z_DN
                | VoxelFlags::HAS_ADJACENT_Z_UP
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
        object.update_adjacency();

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
        object.update_adjacency();

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
}
