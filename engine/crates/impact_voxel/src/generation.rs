//! Generation of spatial voxel distributions.

use crate::{Voxel, VoxelSignedDistance, voxel_types::VoxelType};
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3, point, vector};
use noise::{HybridMulti, MultiFractal, NoiseFn, Simplex};
use ordered_float::OrderedFloat;
use std::array;
use twox_hash::XxHash64;

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
    /// [`Voxel::maximally_outside`].
    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel;
}

/// Represents a signed distance field generator.
///
/// # Note
/// We might not actually want a real signed distance field, because it is hard
/// to modify it efficiently without invalidating distances away from the
/// surface. Instead, it might be better to embrace it as a signed field that
/// has correct distances only close to the surface, as this is what we
/// typically care about.
pub trait SDFGenerator {
    /// Returns the extents of the domain around the center where the signed
    /// distance field can be negative, in voxel grid coordinates.
    fn domain_extents(&self) -> [f64; 3];

    // Computes the signed distance at the given displacement in voxel grid
    // coordinates from the center of the field.
    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64;
}

pub trait VoxelTypeGenerator {
    fn voxel_type_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType;
}

/// Generator for a voxel object from a signed distance field.
#[derive(Clone, Debug)]
pub struct SDFVoxelGenerator<SD, VT> {
    voxel_extent: f64,
    grid_shape: [usize; 3],
    grid_center: Point3<f64>,
    sdf_generator: SD,
    voxel_type_generator: VT,
}

/// Wrapper for a signed distance field generator that adds a multifractal noise
/// term to the output signed distance.
///
/// Note that the resulting field will in general not contain correct distances,
/// so this is best used only for minor perturbations.
#[derive(Clone, Debug)]
pub struct MultifractalNoiseSDFModifier<SD> {
    noise: HybridMulti<Simplex>,
    amplitude: f64,
    sdf_generator: SD,
}

/// Wrapper for a signed distance field generator that performs a stochastic
/// multiscale modification of the output signed distance around the surface.
/// This is done by superimposing a field representing a grid of spheres with
/// randomized radii, which is unioned with the original field aroud the
/// surface. This is repeated for each octave with successively smaller and more
/// numerous spheres.
///
/// See <https://iquilezles.org/articles/fbmsdf/> for more information.
///
/// The output will be a valid signed distance field.
#[derive(Clone, Debug)]
pub struct MultiscaleSphereSDFModifier<SD> {
    octaves: usize,
    frequency: f64,
    persistence: f64,
    inflation: f64,
    smoothness: f64,
    seed: u64,
    sdf_generator: SD,
}

/// Wrapper over two signed distance field generators that outputs the smooth
/// union of the two SDFs.
#[derive(Clone, Debug)]
pub struct SDFUnion<SD1, SD2> {
    smoothness: f64,
    domain_extents: [f64; 3],
    displacement_from_center_to_center_1: Vector3<f64>,
    displacement_from_center_to_center_2: Vector3<f64>,
    sdf_generator_1: SD1,
    sdf_generator_2: SD2,
}

/// Generator for a signed distance field representing a box.
#[derive(Clone, Debug)]
pub struct BoxSDFGenerator {
    half_extents: Vector3<f64>,
}

/// Generator for a signed distance field representing a sphere.
#[derive(Clone, Debug)]
pub struct SphereSDFGenerator {
    radius: f64,
}

/// Generator for a signed "distance" field obtained by thresholding a gradient
/// noise pattern.
#[derive(Clone, Debug)]
pub struct GradientNoiseSDFGenerator {
    extents: [f64; 3],
    noise_frequency: f64,
    noise_threshold: f64,
    noise: Simplex,
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
    noise_frequency: f64,
    noise_scale_for_voxel_type_dim: f64,
    noise: Simplex,
}

impl<SD, VT> SDFVoxelGenerator<SD, VT>
where
    SD: SDFGenerator,
{
    /// Creates a new voxel generator using the given signed distance field
    /// and voxel type generators.
    pub fn new(voxel_extent: f64, sdf_generator: SD, voxel_type_generator: VT) -> Self {
        assert!(voxel_extent > 0.0);

        let sdf_domain_extents = sdf_generator.domain_extents();

        // Make room for a 1-voxel border of empty voxels around the object to so that
        // the surface nets meshing algorithm can correctly interpolate distances at the
        // boundaries
        let grid_shape = sdf_domain_extents.map(|extent| extent.ceil() as usize + 2);

        // The center here is offset by half a grid cell relative to the coordinates
        // in the voxel object to account for the fact that we want to evaluate the
        // SDF at the center of each voxel
        let grid_center = Point3::from(grid_shape.map(|n| 0.5 * (n - 1) as f64));

        Self {
            voxel_extent,
            grid_shape,
            grid_center,
            sdf_generator,
            voxel_type_generator,
        }
    }
}

impl<SD, VT> VoxelGenerator for SDFVoxelGenerator<SD, VT>
where
    SD: SDFGenerator,
    VT: VoxelTypeGenerator,
{
    fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        self.grid_shape
    }

    fn voxel_at_indices(&self, i: usize, j: usize, k: usize) -> Voxel {
        let displacement_from_center = point![i as f64, j as f64, k as f64] - self.grid_center;

        let signed_distance = VoxelSignedDistance::from_f32(
            self.sdf_generator
                .compute_signed_distance(&displacement_from_center) as f32,
        );

        if signed_distance.is_negative() {
            let voxel_type = self.voxel_type_generator.voxel_type_at_indices(i, j, k);
            Voxel::non_empty(voxel_type, signed_distance)
        } else {
            Voxel::empty(signed_distance)
        }
    }
}

impl<SD> MultifractalNoiseSDFModifier<SD> {
    pub fn new(
        sdf_generator: SD,
        octaves: usize,
        frequency: f64,
        lacunarity: f64,
        persistence: f64,
        amplitude: f64,
        seed: u32,
    ) -> Self {
        let noise = HybridMulti::new(seed)
            .set_octaves(octaves)
            .set_frequency(frequency)
            .set_lacunarity(lacunarity)
            .set_persistence(persistence);
        Self {
            noise,
            amplitude,
            sdf_generator,
        }
    }
}

impl<SD> SDFGenerator for MultifractalNoiseSDFModifier<SD>
where
    SD: SDFGenerator,
{
    fn domain_extents(&self) -> [f64; 3] {
        self.sdf_generator.domain_extents()
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64 {
        let signed_distance = self
            .sdf_generator
            .compute_signed_distance(displacement_from_center);

        let noise_point: [f64; 3] = (*displacement_from_center).into();
        let perturbation = self.amplitude * self.noise.get(noise_point);

        signed_distance + perturbation
    }
}

impl<SD> MultiscaleSphereSDFModifier<SD> {
    pub fn new(
        sdf_generator: SD,
        octaves: usize,
        max_scale: f64,
        persistence: f64,
        inflation: f64,
        smoothness: f64,
        seed: u64,
    ) -> Self {
        let frequency = 0.5 / max_scale;

        // Scale inflation and smoothness according to the scale of perturbations
        let inflation = max_scale * inflation;
        let smoothness = max_scale * smoothness;

        Self {
            octaves,
            frequency,
            persistence,
            inflation,
            smoothness,
            seed,
            sdf_generator,
        }
    }

    fn modify_signed_distance(&self, position: &Vector3<f64>, signed_distance: f64) -> f64 {
        /// Rotates with an angle of `2 * pi / golden_ratio` around the axis
        /// `[1, 1, 1]` (to break up the regular grid pattern).
        const ROTATION: UnitQuaternion<f64> = UnitQuaternion::new_unchecked(Quaternion::new(
            0.5381091707820528,
            0.5381091707820528,
            0.5381091707820528,
            -0.36237489008036256,
        ));

        let mut parent_distance = signed_distance;
        let mut position = self.frequency * position;
        let mut scale = 1.0;

        for _ in 0..self.octaves {
            let sphere_grid_distance = scale * self.evaluate_sphere_grid_sdf(&position);

            let intersected_sphere_grid_distance = smooth_sdf_intersection(
                sphere_grid_distance,
                parent_distance - self.inflation * scale,
                self.smoothness * scale,
            );

            parent_distance = smooth_sdf_union(
                intersected_sphere_grid_distance,
                parent_distance,
                self.smoothness * scale,
            );

            position = ROTATION * (position / self.persistence);

            scale *= self.persistence;
        }
        parent_distance
    }

    fn evaluate_sphere_grid_sdf(&self, position: &Vector3<f64>) -> f64 {
        const CORNER_OFFSETS: [Vector3<i32>; 8] = [
            vector![0, 0, 0],
            vector![0, 0, 1],
            vector![0, 1, 0],
            vector![0, 1, 1],
            vector![1, 0, 0],
            vector![1, 0, 1],
            vector![1, 1, 0],
            vector![1, 1, 1],
        ];
        let grid_cell_indices = position.map(|coord| coord.floor() as i32);
        let offset_in_grid_cell = position - grid_cell_indices.cast();

        CORNER_OFFSETS
            .iter()
            .map(|corner_offsets| {
                OrderedFloat(self.evaluate_corner_sphere_sdf(
                    &grid_cell_indices,
                    &offset_in_grid_cell,
                    corner_offsets,
                ))
            })
            .min()
            .unwrap()
            .0
    }

    fn evaluate_corner_sphere_sdf(
        &self,
        grid_cell_indices: &Vector3<i32>,
        offset_in_grid_cell: &Vector3<f64>,
        corner_offsets: &Vector3<i32>,
    ) -> f64 {
        let sphere_radius = self.corner_sphere_radius(grid_cell_indices, corner_offsets);
        let distance_to_sphere_center = (offset_in_grid_cell - corner_offsets.cast()).magnitude();
        distance_to_sphere_center - sphere_radius
    }

    /// Every sphere gets a random radius based on its location in the grid.
    fn corner_sphere_radius(
        &self,
        grid_cell_indices: &Vector3<i32>,
        corner_offsets: &Vector3<i32>,
    ) -> f64 {
        // The maximum radius is half the extent of a grid cell, i.e. 0.5
        const HASH_TO_RADIUS: f64 = 0.5 / u64::MAX as f64;
        let hash = XxHash64::oneshot(
            self.seed,
            bytemuck::bytes_of(&(grid_cell_indices + corner_offsets)),
        );
        HASH_TO_RADIUS * hash as f64
    }
}

impl<SD> SDFGenerator for MultiscaleSphereSDFModifier<SD>
where
    SD: SDFGenerator,
{
    fn domain_extents(&self) -> [f64; 3] {
        self.sdf_generator
            .domain_extents()
            .map(|extent| extent + 5.0 * self.inflation)
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64 {
        let signed_distance = self
            .sdf_generator
            .compute_signed_distance(displacement_from_center);

        self.modify_signed_distance(displacement_from_center, signed_distance)
    }
}

impl<SD1, SD2> SDFUnion<SD1, SD2>
where
    SD1: SDFGenerator,
    SD2: SDFGenerator,
{
    /// Creates a new smooth union wrapper over the given signed distance field
    /// generators, assuming that the centers of the two field domains are
    /// offset by the given offset (in voxels).
    pub fn new(
        sdf_generator_1: SD1,
        sdf_generator_2: SD2,
        center_offsets: [f64; 3],
        smoothness: f64,
    ) -> Self {
        let domain_extents_1 = sdf_generator_1.domain_extents();
        let domain_extents_2 = sdf_generator_2.domain_extents();

        let lower_corner_offsets: [_; 3] = array::from_fn(|dim| {
            center_offsets[dim] + 0.5 * (domain_extents_1[dim] - domain_extents_2[dim])
        });

        let lower_corner: [_; 3] = array::from_fn(|dim| f64::min(0.0, lower_corner_offsets[dim]));

        let domain_extents = array::from_fn(|dim| {
            domain_extents_1[dim].max(domain_extents_2[dim] + lower_corner_offsets[dim])
                - lower_corner[dim]
        });

        let displacement_from_center_to_center_1 =
            array::from_fn(|dim| 0.5 * (domain_extents_1[dim] - domain_extents[dim])).into();

        let displacement_from_center_to_center_2 = array::from_fn(|dim| {
            lower_corner_offsets[dim] + 0.5 * (domain_extents_2[dim] - domain_extents[dim])
        })
        .into();

        Self {
            smoothness,
            domain_extents,
            displacement_from_center_to_center_1,
            displacement_from_center_to_center_2,
            sdf_generator_1,
            sdf_generator_2,
        }
    }
}

impl<SD1, SD2> SDFGenerator for SDFUnion<SD1, SD2>
where
    SD1: SDFGenerator,
    SD2: SDFGenerator,
{
    fn domain_extents(&self) -> [f64; 3] {
        self.domain_extents
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64 {
        let displacement_from_center_1 =
            displacement_from_center + self.displacement_from_center_to_center_1;
        let displacement_from_center_2 =
            displacement_from_center + self.displacement_from_center_to_center_2;

        let signed_distance_1 = self
            .sdf_generator_1
            .compute_signed_distance(&displacement_from_center_1);
        let signed_distance_2 = self
            .sdf_generator_2
            .compute_signed_distance(&displacement_from_center_2);

        smooth_sdf_union(signed_distance_1, signed_distance_2, self.smoothness)
    }
}

impl BoxSDFGenerator {
    /// Creates a new generator for a box with the given extents (in voxels).
    pub fn new(extents: [f64; 3]) -> Self {
        assert!(extents.iter().copied().all(f64::is_sign_positive));
        let half_extents = 0.5 * Vector3::from(extents);
        Self { half_extents }
    }
}

impl SDFGenerator for BoxSDFGenerator {
    fn domain_extents(&self) -> [f64; 3] {
        (2.0 * self.half_extents).into()
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64 {
        let q = displacement_from_center.abs() - self.half_extents;
        q.sup(&Vector3::zeros()).magnitude() + f64::min(q.max(), 0.0)
    }
}

impl SphereSDFGenerator {
    /// Creates a new generator for a sphere with the given radius (in voxels).
    pub fn new(radius: f64) -> Self {
        assert!(radius >= 0.0);
        Self { radius }
    }
}

impl SDFGenerator for SphereSDFGenerator {
    fn domain_extents(&self) -> [f64; 3] {
        [2.0 * self.radius; 3]
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64 {
        displacement_from_center.magnitude() - self.radius
    }
}

impl GradientNoiseSDFGenerator {
    /// Creates a new generator for a gradient noise voxel pattern with the
    /// given extents (in voxels), noise frequency, noise threshold and seed.
    pub fn new(extents: [f64; 3], noise_frequency: f64, noise_threshold: f64, seed: u32) -> Self {
        assert!(extents.iter().copied().all(f64::is_sign_positive));
        let noise = Simplex::new(seed);
        Self {
            extents,
            noise_frequency,
            noise_threshold,
            noise,
        }
    }
}

impl SDFGenerator for GradientNoiseSDFGenerator {
    fn domain_extents(&self) -> [f64; 3] {
        self.extents
    }

    fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64 {
        let noise_point: [f64; 3] = (self.noise_frequency * displacement_from_center).into();
        let noise_value = self.noise.get(noise_point);
        self.noise_threshold - noise_value
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
        noise_frequency: f64,
        voxel_type_frequency: f64,
        seed: u32,
    ) -> Self {
        assert!(!voxel_types.is_empty());

        let noise_scale_for_voxel_type_dim = voxel_type_frequency / voxel_types.len() as f64;

        let noise = Simplex::new(seed);

        Self {
            voxel_types,
            noise_frequency,
            noise_scale_for_voxel_type_dim,
            noise,
        }
    }
}

impl VoxelTypeGenerator for GradientNoiseVoxelTypeGenerator {
    fn voxel_type_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
        let x = i as f64 * self.noise_frequency;
        let y = j as f64 * self.noise_frequency;
        let z = k as f64 * self.noise_frequency;

        self.voxel_types
            .iter()
            .enumerate()
            .map(|(voxel_type_idx, voxel_type)| {
                let voxel_type_coord = voxel_type_idx as f64 * self.noise_scale_for_voxel_type_dim;
                let noise_value = self.noise.get([x, y, z, voxel_type_coord]);
                (noise_value, *voxel_type)
            })
            .max_by_key(|(noise_value, _)| OrderedFloat(*noise_value))
            .unwrap()
            .1
    }
}

fn smooth_sdf_union(distance_1: f64, distance_2: f64, smoothness: f64) -> f64 {
    let h = (0.5 + 0.5 * (distance_2 - distance_1) / smoothness).clamp(0.0, 1.0);
    mix(distance_2, distance_1, h) - smoothness * h * (1.0 - h)
}

#[allow(dead_code)]
fn smooth_sdf_subtraction(distance_1: f64, distance_2: f64, smoothness: f64) -> f64 {
    let h = (0.5 - 0.5 * (distance_2 + distance_1) / smoothness).clamp(0.0, 1.0);
    mix(distance_2, -distance_1, h) + smoothness * h * (1.0 - h)
}

fn smooth_sdf_intersection(distance_1: f64, distance_2: f64, smoothness: f64) -> f64 {
    let h = (0.5 - 0.5 * (distance_2 - distance_1) / smoothness).clamp(0.0, 1.0);
    mix(distance_2, distance_1, h) + smoothness * h * (1.0 - h)
}

fn mix(a: f64, b: f64, factor: f64) -> f64 {
    (1.0 - factor) * a + factor * b
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::voxel_types::VoxelTypeRegistry;
    use arbitrary::{Arbitrary, MaxRecursionReached, Result, Unstructured, size_hint};
    use std::mem;

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    pub enum ArbitraryVoxelTypeGenerator {
        Same(SameVoxelTypeGenerator),
        GradientNoise(GradientNoiseVoxelTypeGenerator),
    }

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    pub enum ArbitrarySDFGenerator {
        Box(BoxSDFGenerator),
        Sphere(SphereSDFGenerator),
        GradientNoise(GradientNoiseSDFGenerator),
    }

    pub type ArbitrarySDFVoxelGenerator =
        SDFVoxelGenerator<ArbitrarySDFGenerator, ArbitraryVoxelTypeGenerator>;

    const MAX_SIZE: usize = 200;

    impl VoxelTypeGenerator for ArbitraryVoxelTypeGenerator {
        fn voxel_type_at_indices(&self, i: usize, j: usize, k: usize) -> VoxelType {
            match self {
                ArbitraryVoxelTypeGenerator::Same(generator) => {
                    generator.voxel_type_at_indices(i, j, k)
                }
                ArbitraryVoxelTypeGenerator::GradientNoise(generator) => {
                    generator.voxel_type_at_indices(i, j, k)
                }
            }
        }
    }

    impl SDFGenerator for ArbitrarySDFGenerator {
        fn domain_extents(&self) -> [f64; 3] {
            match self {
                ArbitrarySDFGenerator::Box(generator) => generator.domain_extents(),
                ArbitrarySDFGenerator::Sphere(generator) => generator.domain_extents(),
                ArbitrarySDFGenerator::GradientNoise(generator) => generator.domain_extents(),
            }
        }

        fn compute_signed_distance(&self, displacement_from_center: &Vector3<f64>) -> f64 {
            match self {
                ArbitrarySDFGenerator::Box(generator) => {
                    generator.compute_signed_distance(displacement_from_center)
                }
                ArbitrarySDFGenerator::Sphere(generator) => {
                    generator.compute_signed_distance(displacement_from_center)
                }
                ArbitrarySDFGenerator::GradientNoise(generator) => {
                    generator.compute_signed_distance(displacement_from_center)
                }
            }
        }
    }

    impl<'a, SD, VT> Arbitrary<'a> for SDFVoxelGenerator<SD, VT>
    where
        SD: SDFGenerator + Arbitrary<'a>,
        VT: VoxelTypeGenerator + Arbitrary<'a>,
    {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_extent = 10.0 * arbitrary_norm_f64(u)?.max(1e-6);
            let sdf_generator: SD = u.arbitrary()?;
            let voxel_type_generator = u.arbitrary()?;
            Ok(Self::new(voxel_extent, sdf_generator, voxel_type_generator))
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            Self::try_size_hint(depth).unwrap_or_default()
        }

        fn try_size_hint(depth: usize) -> Result<(usize, Option<usize>), MaxRecursionReached> {
            size_hint::try_recursion_guard(depth, |depth| {
                Ok(size_hint::and_all(&[
                    (mem::size_of::<i32>(), Some(mem::size_of::<i32>())),
                    SD::size_hint(depth),
                    VT::size_hint(depth),
                ]))
            })
        }
    }

    impl Arbitrary<'_> for BoxSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let extent_x =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f64 + arbitrary_norm_f64(u)?;
            let extent_y =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f64 + arbitrary_norm_f64(u)?;
            let extent_z =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f64 + arbitrary_norm_f64(u)?;
            Ok(Self::new([extent_x, extent_y, extent_z]))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 6 * mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for SphereSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let radius = u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE / 2 - 1) as f64
                + arbitrary_norm_f64(u)?;
            Ok(Self::new(radius))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 2 * mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for GradientNoiseSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let extent_x =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f64 + arbitrary_norm_f64(u)?;
            let extent_y =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f64 + arbitrary_norm_f64(u)?;
            let extent_z =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f64 + arbitrary_norm_f64(u)?;
            let noise_frequency = 0.15 * arbitrary_norm_f64(u)?;
            let noise_threshold = arbitrary_norm_f64(u)?;
            let seed = u.arbitrary()?;
            Ok(Self::new(
                [extent_x, extent_y, extent_z],
                noise_frequency,
                noise_threshold,
                seed,
            ))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 8 * mem::size_of::<usize>() + mem::size_of::<u32>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for SameVoxelTypeGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let idx = u.arbitrary()?;
            Ok(Self::new(VoxelType::from_idx_u8(idx)))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = mem::size_of::<u8>();
            (size, Some(size))
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
            let noise_frequency = 0.15 * arbitrary_norm_f64(u)?;
            let voxel_type_frequency = 0.15 * arbitrary_norm_f64(u)?;
            let seed = u.arbitrary()?;
            Ok(Self::new(
                voxel_types,
                noise_frequency,
                voxel_type_frequency,
                seed,
            ))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let lower_size = mem::size_of::<usize>() + 2 * mem::size_of::<i32>();
            let upper_size =
                lower_size + mem::size_of::<usize>() * (VoxelTypeRegistry::max_n_voxel_types() - 1);
            (lower_size, Some(upper_size))
        }
    }

    fn arbitrary_norm_f64(u: &mut Unstructured<'_>) -> Result<f64> {
        Ok(f64::from(u.int_in_range(0..=1000000)?) / 1000000.0)
    }
}
