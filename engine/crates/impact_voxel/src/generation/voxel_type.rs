//! Generation of voxel type distributions.

use crate::{Voxel, chunks::ChunkedVoxelObject, voxel_types::VoxelType};
use impact_alloc::{AVec, Allocator, avec};
use impact_math::point::Point3A;
use simdnoise::{NoiseBuilder, Settings};
use std::mem;

#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum VoxelTypeGenerator {
    Same(SameVoxelTypeGenerator),
    GradientNoise(GradientNoiseVoxelTypeGenerator),
}

#[derive(Clone, Debug)]
pub struct VoxelTypeGeneratorChunkBuffers<A: Allocator> {
    gradient_noise: AVec<f32, A>,
}

/// Voxel type generator that always returns the same voxel type.
#[derive(Clone, Debug)]
pub struct SameVoxelTypeGenerator {
    voxel_type: VoxelType,
}

/// Voxel type generator that determines voxel types by generating a 4D
/// gradient noise pattern and selecting the voxel type for which the fourth
/// component of the noise is strongest at each location.
#[derive(Clone, Debug)]
pub struct GradientNoiseVoxelTypeGenerator {
    voxel_types: Vec<VoxelType>,
    noise_frequency: f32,
    voxel_type_frequency: f32,
    seed: u32,
}

impl VoxelTypeGenerator {
    pub fn total_buffer_size(&self) -> usize {
        match self {
            Self::Same(_) => 0,
            Self::GradientNoise(generator) => generator.noise_buffer_size(),
        }
    }

    pub fn create_buffers_in<A: Allocator>(&self, alloc: A) -> VoxelTypeGeneratorChunkBuffers<A> {
        let gradient_noise = match self {
            Self::Same(_) => AVec::new_in(alloc),
            Self::GradientNoise(generator) => generator.create_noise_buffer_in(alloc),
        };
        VoxelTypeGeneratorChunkBuffers { gradient_noise }
    }

    pub fn set_voxel_types_for_chunk<A: Allocator>(
        &self,
        voxels: &mut [Voxel],
        buffers: &mut VoxelTypeGeneratorChunkBuffers<A>,
        chunk_origin: &Point3A,
    ) {
        match self {
            Self::Same(generator) => {
                generator.set_voxel_types_for_chunk(voxels);
            }
            Self::GradientNoise(generator) => {
                generator.set_voxel_types_for_chunk(voxels, buffers, chunk_origin);
            }
        }
    }
}

impl From<SameVoxelTypeGenerator> for VoxelTypeGenerator {
    fn from(generator: SameVoxelTypeGenerator) -> Self {
        Self::Same(generator)
    }
}

impl From<GradientNoiseVoxelTypeGenerator> for VoxelTypeGenerator {
    fn from(generator: GradientNoiseVoxelTypeGenerator) -> Self {
        Self::GradientNoise(generator)
    }
}

impl SameVoxelTypeGenerator {
    pub fn new(voxel_type: VoxelType) -> Self {
        Self { voxel_type }
    }

    fn set_voxel_types_for_chunk(&self, voxels: &mut [Voxel]) {
        assert_eq!(voxels.len(), ChunkedVoxelObject::chunk_voxel_count());

        for voxel in voxels {
            voxel.set_voxel_type(self.voxel_type);
        }
    }
}

impl GradientNoiseVoxelTypeGenerator {
    pub fn new(
        voxel_types: Vec<VoxelType>,
        noise_frequency: f32,
        voxel_type_frequency: f32,
        seed: u32,
    ) -> Self {
        assert!(!voxel_types.is_empty());
        Self {
            voxel_types,
            noise_frequency,
            voxel_type_frequency,
            seed,
        }
    }

    fn noise_buffer_size(&self) -> usize {
        self.noise_buffer_len() * mem::size_of::<f32>()
    }

    fn noise_buffer_len(&self) -> usize {
        ChunkedVoxelObject::chunk_voxel_count() * self.voxel_types.len()
    }

    fn create_noise_buffer_in<A: Allocator>(&self, alloc: A) -> AVec<f32, A> {
        avec![in alloc; 0.0; self.noise_buffer_len()]
    }

    fn set_voxel_types_for_chunk<A: Allocator>(
        &self,
        voxels: &mut [Voxel],
        buffers: &mut VoxelTypeGeneratorChunkBuffers<A>,
        chunk_origin: &Point3A,
    ) {
        assert_eq!(voxels.len(), ChunkedVoxelObject::chunk_voxel_count());

        NoiseBuilder::gradient_4d_offset(
            // Warning: We reverse the order of dimensions here because the
            // generated noise is laid out in row-major order
            0.0,
            self.voxel_types.len(),
            chunk_origin.z(),
            ChunkedVoxelObject::chunk_size(),
            chunk_origin.y(),
            ChunkedVoxelObject::chunk_size(),
            chunk_origin.x(),
            ChunkedVoxelObject::chunk_size(),
        )
        .with_freq_4d(
            self.voxel_type_frequency,
            self.noise_frequency,
            self.noise_frequency,
            self.noise_frequency,
        )
        .with_seed(self.seed as i32)
        .generate(&mut buffers.gradient_noise);

        for (voxel, noise_values_for_voxel) in voxels
            .iter_mut()
            .zip(buffers.gradient_noise.chunks(self.voxel_types.len()))
        {
            let mut max_noise = noise_values_for_voxel[0];
            let mut max_idx = 0;
            for (idx, noise) in noise_values_for_voxel.iter().copied().enumerate().skip(1) {
                if noise > max_noise {
                    max_noise = noise;
                    max_idx = idx;
                }
            }
            voxel.set_voxel_type(VoxelType::from_idx(max_idx));
        }
    }
}
