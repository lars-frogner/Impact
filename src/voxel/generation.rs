//! Generation of spatial voxel distributions.

use super::{VoxelGenerator, VoxelType};
use crate::num::Float;
use nalgebra::{point, Point3};
use noise::{NoiseFn, Simplex};

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
    instance_group_height: u32,
}

/// Generator for a voxel configuration obtained by thresholding a gradient
/// noise pattern.
#[derive(Clone, Debug)]
pub struct GradientNoiseVoxelGenerator<F> {
    voxel_type: VoxelType,
    voxel_extent: F,
    size_x: usize,
    size_y: usize,
    size_z: usize,
    noise_distance_scale_x: f64,
    noise_distance_scale_y: f64,
    noise_distance_scale_z: f64,
    noise_threshold: f64,
    noise: Simplex,
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
    pub fn new(
        voxel_type: VoxelType,
        voxel_extent: F,
        n_voxels_across: usize,
        instance_group_height: u32,
    ) -> Self {
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
            instance_group_height,
        }
    }

    /// Returns the position of the sphere center relative to the position of
    /// the origin of the voxel grid.
    pub fn center(&self) -> Point3<F> {
        let center_coord = self.center * self.voxel_extent;
        point![center_coord, center_coord, center_coord]
    }

    /// Returns the radius of the sphere.
    pub fn radius(&self) -> F {
        F::sqrt(self.squared_radius) * self.voxel_extent
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

    fn instance_group_height(&self) -> u32 {
        self.instance_group_height
    }
}

impl<F: Float> GradientNoiseVoxelGenerator<F> {
    /// Creates a new generator for a gradient noise voxel pattern with the
    /// given voxel type, voxel extent and number of voxels in each direction.
    /// The given frequency determines the spatial scale of the noise pattern,
    /// while the given threshold specifies the value the noise pattern must
    /// exceed at a given location to generate a voxel there.
    pub fn new(
        voxel_type: VoxelType,
        voxel_extent: F,
        size_x: usize,
        size_y: usize,
        size_z: usize,
        noise_frequency: f64,
        noise_threshold: f64,
        seed: u32,
    ) -> Self {
        let noise_distance_scale_x = noise_frequency / usize::max(1, size_x) as f64;
        let noise_distance_scale_y = noise_frequency / usize::max(1, size_y) as f64;
        let noise_distance_scale_z = noise_frequency / usize::max(1, size_z) as f64;

        let noise = Simplex::new(seed);

        Self {
            voxel_type,
            voxel_extent,
            size_x,
            size_y,
            size_z,
            noise_distance_scale_x,
            noise_distance_scale_y,
            noise_distance_scale_z,
            noise_threshold,
            noise,
        }
    }
}

impl<F: Float> VoxelGenerator<F> for GradientNoiseVoxelGenerator<F> {
    fn voxel_extent(&self) -> F {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        [self.size_x, self.size_y, self.size_z]
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
        if i < self.size_x && j < self.size_y && k < self.size_z {
            let x = i as f64 * self.noise_distance_scale_x;
            let y = j as f64 * self.noise_distance_scale_y;
            let z = k as f64 * self.noise_distance_scale_z;

            let noise_value = self.noise.get([x, y, z]);

            if noise_value >= self.noise_threshold {
                Some(self.voxel_type)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use arbitrary::{Arbitrary, Result, Unstructured};

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    pub enum ArbitraryVoxelGenerator {
        UniformBox(UniformBoxVoxelGenerator<f64>),
        UniformSphere(UniformSphereVoxelGenerator<f64>),
        GradientNoise(GradientNoiseVoxelGenerator<f64>),
    }

    const MAX_SIZE: usize = 300;

    impl<F: Float> Arbitrary<'_> for UniformBoxVoxelGenerator<F> {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let voxel_type = VoxelType::Default;
            let voxel_extent = F::from_f64(0.25).unwrap();
            let size_x = u.int_in_range(0..=MAX_SIZE)?;
            let size_y = u.int_in_range(0..=MAX_SIZE)?;
            let size_z = u.int_in_range(0..=MAX_SIZE)?;
            Ok(Self::new(voxel_type, voxel_extent, size_x, size_y, size_z))
        }
    }

    impl<F: Float> Arbitrary<'_> for UniformSphereVoxelGenerator<F> {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let voxel_type = VoxelType::Default;
            let voxel_extent = F::from_f64(0.25).unwrap();
            let n_voxels_across = u.int_in_range(1..=MAX_SIZE)?;
            Ok(Self::new(voxel_type, voxel_extent, n_voxels_across, 0))
        }
    }

    impl<'a, F: Float> Arbitrary<'a> for GradientNoiseVoxelGenerator<F> {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_type = VoxelType::Default;
            let voxel_extent = F::from_f64(0.25).unwrap();
            let size_x = u.int_in_range(0..=MAX_SIZE)?;
            let size_y = u.int_in_range(0..=MAX_SIZE)?;
            let size_z = u.int_in_range(0..=MAX_SIZE)?;
            let noise_frequency = 100.0 * f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let noise_threshold = f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let seed = u.arbitrary()?;
            Ok(Self::new(
                voxel_type,
                voxel_extent,
                size_x,
                size_y,
                size_z,
                noise_frequency,
                noise_threshold,
                seed,
            ))
        }
    }
}
