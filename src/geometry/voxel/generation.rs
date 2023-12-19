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

/// Generator for a spherical configuration of identical voxels.
#[derive(Clone, Debug)]
pub struct UniformSphereVoxelGenerator<F> {
    voxel_type: VoxelType,
    voxel_extent: F,
    n_voxels_across: usize,
    center: F,
    squared_radius: F,
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

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
        if i < self.size_x && j < self.size_y && k < self.size_z {
            Some(self.voxel_type)
        } else {
            None
        }
    }
}

impl<F: Float> UniformSphereVoxelGenerator<F> {
    /// Creates a new generator for a uniform sphere with the given voxel type,
    /// voxel extent and number of voxels across the diameter.
    ///
    /// # Panics
    /// If the given number of voxels across is zero.
    pub fn new(voxel_type: VoxelType, voxel_extent: F, n_voxels_across: usize) -> Self {
        assert_ne!(n_voxels_across, 0);

        let center = F::ONE_HALF * F::from_usize(n_voxels_across - 1).unwrap();
        let radius = center + F::ONE_HALF;
        let squared_radius = radius.powi(2);

        Self {
            voxel_type,
            voxel_extent,
            n_voxels_across,
            center,
            squared_radius,
        }
    }
}

impl<F: Float> VoxelGenerator<F> for UniformSphereVoxelGenerator<F> {
    fn voxel_extent(&self) -> F {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        [self.n_voxels_across; 3]
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
        let squared_dist_from_center = (F::from_usize(i).unwrap() - self.center).powi(2)
            + (F::from_usize(j).unwrap() - self.center).powi(2)
            + (F::from_usize(k).unwrap() - self.center).powi(2);

        if squared_dist_from_center <= self.squared_radius {
            Some(self.voxel_type)
        } else {
            None
        }
    }
}
