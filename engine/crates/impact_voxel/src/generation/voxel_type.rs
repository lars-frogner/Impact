//! Generation of voxel type distributions.

use crate::{Voxel, chunks::ChunkedVoxelObject, voxel_types::VoxelType};
use nalgebra::Point3;
use simdnoise::{NoiseBuilder, Settings};

#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum VoxelTypeGenerator {
    Same(SameVoxelTypeGenerator),
    GradientNoise(GradientNoiseVoxelTypeGenerator),
}

#[derive(Clone, Debug)]
pub struct VoxelTypeGeneratorChunkBuffers {
    gradient_noise: Vec<f32>,
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
    pub fn create_buffers(&self) -> VoxelTypeGeneratorChunkBuffers {
        let gradient_noise = match self {
            Self::Same(_) => Vec::new(),
            Self::GradientNoise(generator) => generator.create_noise_buffer(),
        };
        VoxelTypeGeneratorChunkBuffers { gradient_noise }
    }

    pub fn set_voxel_types_for_chunk(
        &self,
        voxels: &mut [Voxel],
        buffers: &mut VoxelTypeGeneratorChunkBuffers,
        chunk_origin: &Point3<f32>,
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

    fn create_noise_buffer(&self) -> Vec<f32> {
        vec![0.0; ChunkedVoxelObject::chunk_voxel_count() * self.voxel_types.len()]
    }

    fn set_voxel_types_for_chunk(
        &self,
        voxels: &mut [Voxel],
        buffers: &mut VoxelTypeGeneratorChunkBuffers,
        chunk_origin: &Point3<f32>,
    ) {
        assert_eq!(voxels.len(), ChunkedVoxelObject::chunk_voxel_count());

        NoiseBuilder::gradient_4d_offset(
            // Warning: We reverse the order of dimensions here because the
            // generated noise is laid out in row-major order
            0.0,
            self.voxel_types.len(),
            chunk_origin.z,
            ChunkedVoxelObject::chunk_size(),
            chunk_origin.y,
            ChunkedVoxelObject::chunk_size(),
            chunk_origin.x,
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
