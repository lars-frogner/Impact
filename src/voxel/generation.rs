//! Generation of spatial voxel distributions.

use crate::voxel::{voxel_types::VoxelType, Voxel};
use nalgebra::{point, Point3};
use noise::{NoiseFn, Simplex};
use ordered_float::OrderedFloat;

/// Represents a voxel generator that provides a voxel type given the voxel
/// indices.
pub trait VoxelGenerator {
    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> f64;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Returns the voxel at the given indices in a voxel grid. If the indices
    /// are outside the bounds of the grid, this should return
    /// [`Voxel::fully_outside`].
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel;
}

pub trait VoxelTypeGenerator {
    fn voxel_type_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType;
}

/// Generator for a box configuration of identical voxels.
#[derive(Clone, Debug)]
pub struct BoxVoxelGenerator<T> {
    voxel_extent: f64,
    size_x: usize,
    size_y: usize,
    size_z: usize,
    voxel_type_generator: T,
}

/// Generator for a spherical configuration of identical voxels.
#[derive(Clone, Debug)]
pub struct SphereVoxelGenerator<T> {
    voxel_extent: f64,
    n_voxels_across: usize,
    center: f64,
    squared_radius: f64,
    voxel_type_generator: T,
}

/// Generator for a voxel configuration obtained by thresholding a gradient
/// noise pattern.
#[derive(Clone, Debug)]
pub struct GradientNoiseVoxelGenerator<T> {
    voxel_extent: f64,
    size_x: usize,
    size_y: usize,
    size_z: usize,
    noise_distance_scale_x: f64,
    noise_distance_scale_y: f64,
    noise_distance_scale_z: f64,
    noise_threshold: f64,
    noise: Simplex,
    voxel_type_generator: T,
}

#[derive(Clone, Debug)]
pub struct SameVoxelTypeGenerator {
    voxel_type: VoxelType,
}

#[derive(Clone, Debug)]
pub struct GradientNoiseVoxelTypeGenerator {
    voxel_types: Vec<VoxelType>,
    noise_distance_scale_x: f64,
    noise_distance_scale_y: f64,
    noise_distance_scale_z: f64,
    noise_distance_scale_voxel_type_dim: f64,
    noise: Simplex,
}

impl<T> BoxVoxelGenerator<T> {
    /// Creates a new generator for a box with the given voxel extent and number
    /// of voxels in each direction, using the given voxel type generator.
    pub fn new(
        voxel_extent: f64,
        size_x: usize,
        size_y: usize,
        size_z: usize,
        voxel_type_generator: T,
    ) -> Self {
        Self {
            voxel_type_generator,
            voxel_extent,
            size_x,
            size_y,
            size_z,
        }
    }
}

impl<T: VoxelTypeGenerator> VoxelGenerator for BoxVoxelGenerator<T> {
    fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        [self.size_x, self.size_y, self.size_z]
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel {
        if i < self.size_x && j < self.size_y && k < self.size_z {
            let voxel_type = self.voxel_type_generator.voxel_type_at_indices(i, j, k);
            Voxel::fully_inside(voxel_type)
        } else {
            Voxel::fully_outside()
        }
    }
}

impl<T> SphereVoxelGenerator<T> {
    /// Creates a new generator for a sphere with the given voxel extent and
    /// number of voxels across the diameter, using the given voxel type
    /// generator.
    ///
    /// # Panics
    /// If the given number of voxels across is zero.
    pub fn new(voxel_extent: f64, n_voxels_across: usize, voxel_type_generator: T) -> Self {
        assert_ne!(n_voxels_across, 0);

        let center = 0.5 * (n_voxels_across - 1) as f64;
        let radius = center + 0.5;
        let squared_radius = radius.powi(2);

        Self {
            voxel_type_generator,
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

impl<T: VoxelTypeGenerator> VoxelGenerator for SphereVoxelGenerator<T> {
    fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        [self.n_voxels_across; 3]
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel {
        let squared_dist_from_center = (i as f64 - self.center).powi(2)
            + (j as f64 - self.center).powi(2)
            + (k as f64 - self.center).powi(2);

        if squared_dist_from_center <= self.squared_radius {
            let voxel_type = self.voxel_type_generator.voxel_type_at_indices(i, j, k);
            Voxel::fully_inside(voxel_type)
        } else {
            Voxel::fully_outside()
        }
    }
}

impl<T> GradientNoiseVoxelGenerator<T> {
    /// Creates a new generator for a gradient noise voxel pattern with the
    /// given voxel extent and number of voxels in each direction, using the
    /// given voxel type generator. The given frequency determines the spatial
    /// scale of the noise pattern, while the given threshold specifies the
    /// value the noise pattern must exceed at a given location to generate a
    /// voxel there.
    pub fn new(
        voxel_extent: f64,
        size_x: usize,
        size_y: usize,
        size_z: usize,
        noise_frequency: f64,
        noise_threshold: f64,
        seed: u32,
        voxel_type_generator: T,
    ) -> Self {
        let noise_distance_scale_x = noise_frequency / usize::max(1, size_x) as f64;
        let noise_distance_scale_y = noise_frequency / usize::max(1, size_y) as f64;
        let noise_distance_scale_z = noise_frequency / usize::max(1, size_z) as f64;

        let noise = Simplex::new(seed);

        Self {
            voxel_extent,
            size_x,
            size_y,
            size_z,
            noise_distance_scale_x,
            noise_distance_scale_y,
            noise_distance_scale_z,
            noise_threshold,
            noise,
            voxel_type_generator,
        }
    }
}

impl<T: VoxelTypeGenerator> VoxelGenerator for GradientNoiseVoxelGenerator<T> {
    fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        [self.size_x, self.size_y, self.size_z]
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel {
        if i < self.size_x && j < self.size_y && k < self.size_z {
            let x = i as f64 * self.noise_distance_scale_x;
            let y = j as f64 * self.noise_distance_scale_y;
            let z = k as f64 * self.noise_distance_scale_z;

            let noise_value = self.noise.get([x, y, z]);

            if noise_value >= self.noise_threshold {
                let voxel_type = self.voxel_type_generator.voxel_type_at_indices(i, j, k);
                Voxel::fully_inside(voxel_type)
            } else {
                Voxel::fully_outside()
            }
        } else {
            Voxel::fully_outside()
        }
    }
}

impl SameVoxelTypeGenerator {
    pub fn new(voxel_type: VoxelType) -> Self {
        Self { voxel_type }
    }
}

impl VoxelTypeGenerator for SameVoxelTypeGenerator {
    fn voxel_type_at_indices(&self, _i: usize, _j: usize, _k: usize) -> VoxelType {
        self.voxel_type
    }
}

impl GradientNoiseVoxelTypeGenerator {
    pub fn new(
        voxel_types: Vec<VoxelType>,
        voxel_type_frequency: f64,
        noise_distance_scale_x: f64,
        noise_distance_scale_y: f64,
        noise_distance_scale_z: f64,
        seed: u32,
    ) -> Self {
        assert!(!voxel_types.is_empty());

        let noise_distance_scale_voxel_type_dim = voxel_type_frequency / voxel_types.len() as f64;

        let noise = Simplex::new(seed);

        Self {
            voxel_types,
            noise_distance_scale_x,
            noise_distance_scale_y,
            noise_distance_scale_z,
            noise_distance_scale_voxel_type_dim,
            noise,
        }
    }
}

impl VoxelTypeGenerator for GradientNoiseVoxelTypeGenerator {
    fn voxel_type_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
        let x = i as f64 * self.noise_distance_scale_x;
        let y = j as f64 * self.noise_distance_scale_y;
        let z = k as f64 * self.noise_distance_scale_z;

        self.voxel_types
            .iter()
            .enumerate()
            .map(|(voxel_type_idx, voxel_type)| {
                let voxel_type_coord =
                    voxel_type_idx as f64 * self.noise_distance_scale_voxel_type_dim;
                let noise_value = self.noise.get([x, y, z, voxel_type_coord]);
                (noise_value, *voxel_type)
            })
            .max_by_key(|(noise_value, _)| OrderedFloat(*noise_value))
            .unwrap()
            .1
    }
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use crate::voxel::voxel_types::VoxelTypeRegistry;

    use super::*;
    use arbitrary::{size_hint, Arbitrary, Result, Unstructured};
    use std::mem;

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    pub enum ArbitraryVoxelGenerator {
        Box(BoxVoxelGenerator<GradientNoiseVoxelTypeGenerator>),
        Sphere(SphereVoxelGenerator<GradientNoiseVoxelTypeGenerator>),
        GradientNoise(GradientNoiseVoxelGenerator<GradientNoiseVoxelTypeGenerator>),
    }

    const MAX_SIZE: usize = 300;

    impl<'a, T> Arbitrary<'a> for BoxVoxelGenerator<T>
    where
        T: Arbitrary<'a>,
    {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_extent = 0.25;
            let size_x = u.int_in_range(0..=MAX_SIZE)?;
            let size_y = u.int_in_range(0..=MAX_SIZE)?;
            let size_z = u.int_in_range(0..=MAX_SIZE)?;
            let voxel_type_generator = u.arbitrary()?;
            Ok(Self::new(
                voxel_extent,
                size_x,
                size_y,
                size_z,
                voxel_type_generator,
            ))
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            let size = 3 * mem::size_of::<usize>();
            size_hint::recursion_guard(depth, |depth| {
                size_hint::and((size, Some(size)), T::size_hint(depth))
            })
        }
    }

    impl<'a, T> Arbitrary<'a> for SphereVoxelGenerator<T>
    where
        T: Arbitrary<'a>,
    {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_extent = 0.25;
            let n_voxels_across = u.int_in_range(1..=MAX_SIZE)?;
            let voxel_type_generator = u.arbitrary()?;
            Ok(Self::new(
                voxel_extent,
                n_voxels_across,
                voxel_type_generator,
            ))
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            let size = mem::size_of::<usize>();
            size_hint::recursion_guard(depth, |depth| {
                size_hint::and((size, Some(size)), T::size_hint(depth))
            })
        }
    }

    impl<'a, T> Arbitrary<'a> for GradientNoiseVoxelGenerator<T>
    where
        T: Arbitrary<'a>,
    {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_extent = 0.25;
            let size_x = u.int_in_range(0..=MAX_SIZE)?;
            let size_y = u.int_in_range(0..=MAX_SIZE)?;
            let size_z = u.int_in_range(0..=MAX_SIZE)?;
            let noise_frequency = 100.0 * f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let noise_threshold = f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let seed = u.arbitrary()?;
            let voxel_type_generator = u.arbitrary()?;
            Ok(Self::new(
                voxel_extent,
                size_x,
                size_y,
                size_z,
                noise_frequency,
                noise_threshold,
                seed,
                voxel_type_generator,
            ))
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            let size =
                3 * mem::size_of::<usize>() + 2 * mem::size_of::<i32>() + mem::size_of::<u32>();
            size_hint::recursion_guard(depth, |depth| {
                size_hint::and((size, Some(size)), T::size_hint(depth))
            })
        }
    }

    impl Arbitrary<'_> for GradientNoiseVoxelTypeGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let mut voxel_types: Vec<_> = (0..VoxelTypeRegistry::max_n_voxel_types())
                .map(VoxelType::from_idx)
                .collect();
            for _ in 0..u.int_in_range(0..=voxel_types.len() - 1)? {
                voxel_types.swap_remove(u.int_in_range(0..=voxel_types.len() - 1)?);
            }
            let voxel_type_frequency = 100.0 * f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let noise_distance_scale_x =
                100.0 * f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let noise_distance_scale_y =
                100.0 * f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let noise_distance_scale_z =
                100.0 * f64::from(u.int_in_range(0..=1000000)?) / 1000000.0;
            let seed = u.arbitrary()?;
            Ok(Self::new(
                voxel_types,
                voxel_type_frequency,
                noise_distance_scale_x,
                noise_distance_scale_y,
                noise_distance_scale_z,
                seed,
            ))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let lower_size = mem::size_of::<usize>() + 4 * mem::size_of::<i32>();
            let upper_size =
                lower_size + mem::size_of::<usize>() * (VoxelTypeRegistry::max_n_voxel_types() - 1);
            (lower_size, Some(upper_size))
        }
    }
}
