//! Generation of spatial voxel distributions.

use super::{VoxelGenerator, VoxelType};
use crate::num::Float;

/// Generator for a box configuration of identical voxels.
#[derive(Clone, Debug)]
pub struct UniformBoxVoxelGenerator<F> {
    voxel_type: VoxelType,
    voxel_extent: F,
    size_x: usize,
    size_y: usize,
    size_z: usize,
}

impl<F: Float> UniformBoxVoxelGenerator<F> {
    /// Creates a new generator for a uniform box with the given voxel type,
    /// voxel extent and number of voxels in each direction.
    pub fn new(
        voxel_type: VoxelType,
        voxel_extent: F,
        size_x: usize,
        size_y: usize,
        size_z: usize,
    ) -> Self {
        Self {
            voxel_type,
            voxel_extent,
            size_x,
            size_y,
            size_z,
        }
    }
}

impl<F: Float> VoxelGenerator<F> for UniformBoxVoxelGenerator<F> {
    fn voxel_extent(&self) -> F {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        [self.size_x, self.size_y, self.size_z]
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
        if i < self.size_x && j < self.size_y && k < self.size_z {
            self.voxel_type
        } else {
            VoxelType::Empty
        }
    }
}
