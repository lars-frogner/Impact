//! Generation of spatial voxel distributions.

use super::{VoxelGenerator, VoxelType};
use nalgebra::{point, Point3};
use noise::{NoiseFn, Simplex};

/// Generator for a box configuration of identical voxels.
#[derive(Clone, Debug)]
pub struct UniformBoxVoxelGenerator {
    voxel_type: VoxelType,
    voxel_extent: f64,
    size_x: usize,
    size_y: usize,
    size_z: usize,
}

/// Generator for a spherical configuration of identical voxels.
#[derive(Clone, Debug)]
pub struct UniformSphereVoxelGenerator {
    voxel_type: VoxelType,
    voxel_extent: f64,
    n_voxels_across: usize,
    center: f64,
    squared_radius: f64,
}

/// Generator for a voxel configuration obtained by thresholding a gradient
/// noise pattern.
#[derive(Clone, Debug)]
pub struct GradientNoiseVoxelGenerator {
    voxel_type: VoxelType,
    voxel_extent: f64,
    size_x: usize,
    size_y: usize,
    size_z: usize,
    noise_distance_scale_x: f64,
    noise_distance_scale_y: f64,
    noise_distance_scale_z: f64,
    noise_threshold: f64,
    noise: Simplex,
}

impl UniformBoxVoxelGenerator {
    /// Creates a new generator for a uniform box with the given voxel type,
    /// voxel extent and number of voxels in each direction.
    pub fn new(
        voxel_type: VoxelType,
        voxel_extent: f64,
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

impl VoxelGenerator for UniformBoxVoxelGenerator {
    fn voxel_extent(&self) -> f64 {
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

impl UniformSphereVoxelGenerator {
    /// Creates a new generator for a uniform sphere with the given voxel type,
    /// voxel extent and number of voxels across the diameter.
    ///
    /// # Panics
    /// If the given number of voxels across is zero.
    pub fn new(voxel_type: VoxelType, voxel_extent: f64, n_voxels_across: usize) -> Self {
        assert_ne!(n_voxels_across, 0);

        let center = 0.5 * (n_voxels_across - 1) as f64;
        let radius = center + 0.5;
        let squared_radius = radius.powi(2);

        Self {
            voxel_type,
            voxel_extent,
            n_voxels_across,
            center,
            squared_radius,
        }
    }

    /// Returns the position of the sphere center relative to the position of
    /// the origin of the voxel grid.
    pub fn center(&self) -> Point3<f64> {
        let center_coord = self.center * self.voxel_extent;
        point![center_coord, center_coord, center_coord]
    }

    /// Returns the radius of the sphere.
    pub fn radius(&self) -> f64 {
        f64::sqrt(self.squared_radius) * self.voxel_extent
    }
}

impl VoxelGenerator for UniformSphereVoxelGenerator {
    fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        [self.n_voxels_across; 3]
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Option<VoxelType> {
        let squared_dist_from_center = (i as f64 - self.center).powi(2)
            + (j as f64 - self.center).powi(2)
            + (k as f64 - self.center).powi(2);

        if squared_dist_from_center <= self.squared_radius {
            Some(self.voxel_type)
        } else {
            None
        }
    }
}

impl GradientNoiseVoxelGenerator {
    /// Creates a new generator for a gradient noise voxel pattern with the
    /// given voxel type, voxel extent and number of voxels in each direction.
    /// The given frequency determines the spatial scale of the noise pattern,
    /// while the given threshold specifies the value the noise pattern must
    /// exceed at a given location to generate a voxel there.
    pub fn new(
        voxel_type: VoxelType,
        voxel_extent: f64,
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

impl VoxelGenerator for GradientNoiseVoxelGenerator {
    fn voxel_extent(&self) -> f64 {
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
    use std::mem;

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    pub enum ArbitraryVoxelGenerator {
        UniformBox(UniformBoxVoxelGenerator),
        UniformSphere(UniformSphereVoxelGenerator),
        GradientNoise(GradientNoiseVoxelGenerator),
    }

    const MAX_SIZE: usize = 300;

    impl Arbitrary<'_> for UniformBoxVoxelGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let voxel_type = VoxelType::Default;
            let voxel_extent = 0.25;
            let size_x = u.int_in_range(0..=MAX_SIZE)?;
            let size_y = u.int_in_range(0..=MAX_SIZE)?;
            let size_z = u.int_in_range(0..=MAX_SIZE)?;
            Ok(Self::new(voxel_type, voxel_extent, size_x, size_y, size_z))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 3 * mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for UniformSphereVoxelGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let voxel_type = VoxelType::Default;
            let voxel_extent = 0.25;
            let n_voxels_across = u.int_in_range(1..=MAX_SIZE)?;
            Ok(Self::new(voxel_type, voxel_extent, n_voxels_across))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for GradientNoiseVoxelGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let voxel_type = VoxelType::Default;
            let voxel_extent = 0.25;
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

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size =
                3 * mem::size_of::<usize>() + 2 * mem::size_of::<f64>() + mem::size_of::<u32>();
            (size, Some(size))
        }
    }
}
