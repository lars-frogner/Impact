//!

use crate::{
    num::Float,
    voxel::{VoxelGenerator, VoxelType},
};
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct ChunkedVoxelObject {
    voxel_extent: f64,
    n_superchunks_per_axis: usize,
    occupied_chunks: [Range<usize>; 3],
    superchunks: Vec<VoxelSuperchunk>,
}

#[derive(Clone, Debug)]
pub struct VoxelSuperchunk {
    chunks: SuperchunkChunks,
}

#[derive(Clone, Debug)]
enum SuperchunkChunks {
    None,
    Same(Voxel),
    Different(Vec<VoxelChunk>),
}

#[derive(Clone, Debug)]
pub struct VoxelChunk {
    voxels: ChunkVoxels,
}

#[derive(Clone, Debug)]
enum ChunkVoxels {
    None,
    Same(Voxel),
    Different(Vec<Voxel>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voxel {
    property_id: PropertyID,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropertyID(u8);

const LOG2_CHUNK_SIZE: usize = 4;
const CHUNK_SIZE: usize = 1 << LOG2_CHUNK_SIZE;
const CHUNK_VOXEL_COUNT: usize = CHUNK_SIZE.pow(3);

const LOG2_SUPERCHUNK_SIZE: usize = 3;
const SUPERCHUNK_SIZE: usize = 1 << LOG2_SUPERCHUNK_SIZE;
const SUPERCHUNK_SIZE_IN_VOXELS: usize = SUPERCHUNK_SIZE * CHUNK_SIZE;
const SUPERCHUNK_CHUNK_COUNT: usize = SUPERCHUNK_SIZE.pow(3);

const SUPERCHUNK_IDX_SHIFT: usize = 3 * (LOG2_CHUNK_SIZE + LOG2_SUPERCHUNK_SIZE);
const CHUNK_IDX_SHIFT: usize = 3 * LOG2_CHUNK_SIZE;
const CHUNK_IDX_MASK: usize = 1 << (3 * LOG2_SUPERCHUNK_SIZE - 1);
const VOXEL_IDX_MASK: usize = 1 << (3 * LOG2_CHUNK_SIZE - 1);

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

        let mut chunk_storage = Vec::new();
        let mut voxel_storage = Vec::new();

        let mut occupied_chunks_i = REVERSED_MAX_RANGE;
        let mut occupied_chunks_j = REVERSED_MAX_RANGE;
        let mut occupied_chunks_k = REVERSED_MAX_RANGE;

        for superchunk_k in 0..n_superchunks_per_axis {
            for superchunk_j in 0..n_superchunks_per_axis {
                for superchunk_i in 0..n_superchunks_per_axis {
                    let (superchunk, occupied_chunks, chunk_storage_, voxel_storage_) =
                        VoxelSuperchunk::generate(
                            generator,
                            [superchunk_i, superchunk_j, superchunk_k],
                            chunk_storage,
                            voxel_storage,
                        );

                    occupied_chunks_i.start = occupied_chunks_i.start.min(occupied_chunks[0].start);
                    occupied_chunks_i.end = occupied_chunks_i.end.max(occupied_chunks[0].end);
                    occupied_chunks_j.start = occupied_chunks_j.start.min(occupied_chunks[1].start);
                    occupied_chunks_j.end = occupied_chunks_j.end.max(occupied_chunks[1].end);
                    occupied_chunks_k.start = occupied_chunks_k.start.min(occupied_chunks[2].start);
                    occupied_chunks_k.end = occupied_chunks_k.end.max(occupied_chunks[2].end);

                    // If the superchunk didn't need the storages, we can reuse
                    // them for the next superchunk
                    chunk_storage = chunk_storage_;
                    voxel_storage = voxel_storage_;

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
            .map(VoxelSuperchunk::stored_voxel_count)
            .sum()
    }
}

impl VoxelSuperchunk {
    fn generate<G, F>(
        generator: &G,
        superchunk_indices: [usize; 3],
        mut chunk_storage: Vec<VoxelChunk>,
        mut voxel_storage: Vec<Voxel>,
    ) -> (Self, [Range<usize>; 3], Vec<VoxelChunk>, Vec<Voxel>)
    where
        G: VoxelGenerator<F>,
        F: Float,
    {
        let mut first_voxel: Option<Voxel> = None;
        let mut is_uniform = true;

        chunk_storage.clear();
        chunk_storage.reserve_exact(SUPERCHUNK_CHUNK_COUNT);

        // Note: These are global chunk indices, not the chunk indices within
        // the current superchunk
        let start_chunk_indices = superchunk_indices.map(|idx| idx * SUPERCHUNK_SIZE);

        let mut occupied_chunks_i = REVERSED_MAX_RANGE;
        let mut occupied_chunks_j = REVERSED_MAX_RANGE;
        let mut occupied_chunks_k = REVERSED_MAX_RANGE;

        for chunk_k in start_chunk_indices[2]..start_chunk_indices[2] + SUPERCHUNK_SIZE {
            for chunk_j in start_chunk_indices[1]..start_chunk_indices[1] + SUPERCHUNK_SIZE {
                for chunk_i in start_chunk_indices[0]..start_chunk_indices[0] + SUPERCHUNK_SIZE {
                    let (chunk, voxel_storage_) =
                        VoxelChunk::generate(generator, [chunk_i, chunk_j, chunk_k], voxel_storage);

                    // If the chunk didn't need the storage, we can reuse it for
                    // the next chunk
                    voxel_storage = voxel_storage_;

                    if is_uniform {
                        match (&first_voxel, &chunk.voxels) {
                            (Some(first_voxel), ChunkVoxels::None) => {
                                is_uniform = first_voxel.is_empty();
                            }
                            (Some(first_voxel), ChunkVoxels::Same(voxel)) => {
                                is_uniform = first_voxel == voxel;
                            }
                            (_, ChunkVoxels::Different(_)) => {
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

                    chunk_storage.push(chunk);
                }
            }
        }

        let occupied_chunks = [occupied_chunks_i, occupied_chunks_j, occupied_chunks_k];

        if is_uniform {
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
                chunk_storage,
                voxel_storage,
            )
        } else {
            (
                Self {
                    chunks: SuperchunkChunks::Different(chunk_storage),
                },
                occupied_chunks,
                Vec::new(),
                voxel_storage,
            )
        }
    }

    fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    fn stored_voxel_count(&self) -> usize {
        match &self.chunks {
            SuperchunkChunks::None => 0,
            SuperchunkChunks::Same(_) => 1,
            SuperchunkChunks::Different(chunks) => {
                chunks.iter().map(VoxelChunk::stored_voxel_count).sum()
            }
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
        mut voxel_storage: Vec<Voxel>,
    ) -> (Self, Vec<Voxel>)
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
            return (
                Self {
                    voxels: ChunkVoxels::None,
                },
                voxel_storage,
            );
        }

        let first_voxel = Voxel::new(
            generator
                .voxel_at_indices(origin[0], origin[1], origin[2])
                .map_or_else(PropertyID::empty, PropertyID::from_voxel_type),
        );
        let mut is_uniform = true;

        voxel_storage.clear();
        voxel_storage.reserve_exact(CHUNK_VOXEL_COUNT);

        for k in origin[2]..origin[2] + CHUNK_SIZE {
            for j in origin[1]..origin[1] + CHUNK_SIZE {
                for i in origin[0]..origin[0] + CHUNK_SIZE {
                    let voxel = Voxel::new(
                        generator
                            .voxel_at_indices(i, j, k)
                            .map_or_else(PropertyID::empty, PropertyID::from_voxel_type),
                    );
                    if is_uniform && voxel != first_voxel {
                        is_uniform = false;
                    }
                    voxel_storage.push(voxel);
                }
            }
        }

        if is_uniform {
            (
                Self {
                    voxels: if first_voxel.is_empty() {
                        ChunkVoxels::None
                    } else {
                        ChunkVoxels::Same(first_voxel)
                    },
                },
                voxel_storage,
            )
        } else {
            (
                Self {
                    voxels: ChunkVoxels::Different(voxel_storage),
                },
                Vec::new(),
            )
        }
    }

    fn is_empty(&self) -> bool {
        self.voxels.is_empty()
    }

    fn stored_voxel_count(&self) -> usize {
        match &self.voxels {
            ChunkVoxels::None => 0,
            ChunkVoxels::Same(_) => 1,
            ChunkVoxels::Different(_) => CHUNK_VOXEL_COUNT,
        }
    }
}

impl ChunkVoxels {
    fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Voxel {
    pub const fn new(property_id: PropertyID) -> Self {
        Self { property_id }
    }

    pub const fn empty() -> Self {
        Self {
            property_id: PropertyID::empty(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.property_id == PropertyID::empty()
    }
}

impl PropertyID {
    pub const fn empty() -> Self {
        Self(u8::MAX)
    }

    pub const fn from_voxel_type(voxel_type: VoxelType) -> Self {
        Self(voxel_type as u8)
    }
}

const fn superchunk_idx(global_voxel_idx: usize) -> usize {
    global_voxel_idx >> SUPERCHUNK_IDX_SHIFT
}

const fn chunk_idx(global_voxel_idx: usize) -> usize {
    (global_voxel_idx >> CHUNK_IDX_SHIFT) & CHUNK_IDX_MASK
}

const fn voxel_idx(global_voxel_idx: usize) -> usize {
    global_voxel_idx & VOXEL_IDX_MASK
}

#[cfg(test)]
mod test {
    use super::*;

    struct BoxVoxelGenerator {
        shape: [usize; 3],
        offset: [usize; 3],
        voxel_type: Option<VoxelType>,
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

    impl VoxelGenerator<f64> for BoxVoxelGenerator {
        fn voxel_extent(&self) -> f64 {
            0.25
        }

        fn grid_shape(&self) -> [usize; 3] {
            self.shape
        }

        fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
            if i >= self.offset[0]
                && i < self.shape[0]
                && j >= self.offset[1]
                && j < self.shape[1]
                && k >= self.offset[2]
                && k < self.shape[2]
            {
                self.voxel_type
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
        let generator =
            BoxVoxelGenerator::offset_with_default([2 * CHUNK_SIZE; 3], [CHUNK_SIZE; 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        assert_eq!(object.n_superchunks_per_axis(), 1);
        assert_eq!(object.full_grid_size(), SUPERCHUNK_SIZE_IN_VOXELS);
        assert_eq!(object.occupied_range(0), CHUNK_SIZE..2 * CHUNK_SIZE);
        assert_eq!(object.occupied_range(1), CHUNK_SIZE..2 * CHUNK_SIZE);
        assert_eq!(object.occupied_range(2), CHUNK_SIZE..2 * CHUNK_SIZE);
        assert_eq!(object.stored_voxel_count(), 1);
    }
}
