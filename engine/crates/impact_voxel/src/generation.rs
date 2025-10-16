//! Generation of spatial voxel distributions.

pub mod sdf;
pub mod voxel_type;

use crate::{
    Voxel, VoxelSignedDistance,
    chunks::{ChunkedVoxelObject, LoopForChunkVoxels},
    voxel_types::VoxelType,
};
use allocator_api2::vec::Vec as AVec;
use impact_geometry::AxisAlignedBox;
use nalgebra::{Point3, Vector3};
use sdf::{SDFGenerator, SDFGeneratorChunkBuffers};
use voxel_type::{VoxelTypeGenerator, VoxelTypeGeneratorChunkBuffers};

/// Represents a voxel generator that provides voxels for a chunked voxel
/// object.
pub trait VoxelGenerator {
    type ChunkGenerationBuffers;

    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> f64;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// Creates temporary buffers used when generating chunks of voxels. They
    /// are meant to be reused across generation calls.
    fn create_buffers(&self) -> Self::ChunkGenerationBuffers;

    /// Generates voxels for a single chunks with the given chunk origin (global
    /// voxel object indices of the lower chunk corner). The voxels are appended
    /// to the given vector.
    fn generate_chunk(
        &self,
        buffers: &mut Self::ChunkGenerationBuffers,
        voxels: &mut Vec<Voxel>,
        chunk_origin: &[usize; 3],
    );
}

/// Generator for a voxel object from a signed distance field.
#[derive(Clone, Debug)]
pub struct SDFVoxelGenerator {
    voxel_extent: f64,
    grid_shape: [usize; 3],
    shifted_grid_center: Point3<f32>,
    sdf_generator: SDFGenerator,
    voxel_type_generator: VoxelTypeGenerator,
}

#[derive(Clone, Debug)]
pub struct SDFVoxelGeneratorChunkBuffers {
    sdf: SDFGeneratorChunkBuffers,
    voxel_type: VoxelTypeGeneratorChunkBuffers,
}

impl SDFVoxelGenerator {
    /// Creates a new voxel generator using the given signed distance field
    /// and voxel type generators.
    pub fn new(
        voxel_extent: f64,
        sdf_generator: SDFGenerator,
        voxel_type_generator: VoxelTypeGenerator,
    ) -> Self {
        assert!(voxel_extent > 0.0);

        let sdf_domain = sdf_generator.domain();
        let sdf_domain_extents: [_; 3] = sdf_domain.extents().into();

        if sdf_domain_extents.contains(&0.0) {
            return Self {
                voxel_extent,
                grid_shape: [0; 3],
                shifted_grid_center: [-0.5; 3].into(),
                sdf_generator,
                voxel_type_generator,
            };
        }

        // Make room for a border of empty voxels around the object to so that
        // the surface nets meshing algorithm can correctly interpolate
        // distances at the boundaries
        let grid_shape = sdf_domain_extents.map(|extent| {
            let extent = extent.ceil() as usize;
            // Add a one-voxel border on each side
            extent + 2
        });

        let grid_center_relative_to_domain_lower_corner =
            Point3::from(grid_shape.map(|n| 0.5 * n as f32));

        // Since the domain can be translated relative to the origin of the root
        // SDF coordinate space, we subtract the domain center to get the grid
        // center relative to the origin
        let grid_center_relative_to_sdf_origin =
            grid_center_relative_to_domain_lower_corner - sdf_domain.center().coords;

        // The center here is offset by half a grid cell relative to the coordinates
        // in the voxel object to account for the fact that we want to evaluate the
        // SDF at the center of each voxel
        let shifted_grid_center_relative_to_sdf_origin =
            grid_center_relative_to_sdf_origin.map(|coord| coord - 0.5);

        Self {
            voxel_extent,
            grid_shape,
            shifted_grid_center: shifted_grid_center_relative_to_sdf_origin,
            sdf_generator,
            voxel_type_generator,
        }
    }

    /// Returns the center of the voxel grid in the root SDF coordinate space.
    /// The coordinates are in whole voxels.
    pub fn grid_center(&self) -> Point3<f32> {
        self.shifted_grid_center.map(|coord| coord + 0.5) // Unshift
    }
}

impl VoxelGenerator for SDFVoxelGenerator {
    type ChunkGenerationBuffers = SDFVoxelGeneratorChunkBuffers;

    fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    fn grid_shape(&self) -> [usize; 3] {
        self.grid_shape
    }

    fn create_buffers(&self) -> Self::ChunkGenerationBuffers {
        SDFVoxelGeneratorChunkBuffers {
            sdf: self.sdf_generator.create_buffers(),
            voxel_type: self.voxel_type_generator.create_buffers(),
        }
    }

    fn generate_chunk(
        &self,
        buffers: &mut Self::ChunkGenerationBuffers,
        voxels: &mut Vec<Voxel>,
        chunk_origin: &[usize; 3],
    ) {
        if self.sdf_generator.is_empty()
            || chunk_origin
                .iter()
                .zip(self.grid_shape)
                .any(|(&origin, size)| origin >= size)
        {
            voxels.resize(
                voxels.len() + ChunkedVoxelObject::chunk_voxel_count(),
                Voxel::maximally_outside(),
            );
            return;
        }

        let chunk_origin_in_root_space =
            Point3::from(chunk_origin.map(|idx| idx as f32)) - self.shifted_grid_center.coords;

        let chunk_aabb_in_root_space = AxisAlignedBox::new(
            chunk_origin_in_root_space,
            chunk_origin_in_root_space + Vector3::repeat(ChunkedVoxelObject::chunk_size() as f32),
        );

        self.sdf_generator
            .compute_signed_distances_for_chunk(&mut buffers.sdf, &chunk_aabb_in_root_space);

        let signed_distances = buffers.sdf.final_signed_distances();

        let start_voxel_idx = voxels.len();
        voxels.reserve(ChunkedVoxelObject::chunk_voxel_count());

        let mut chunk_is_empty = true;

        LoopForChunkVoxels::over_all().execute_with_linear_idx(
            &mut |&[i_in_chunk, j_in_chunk, k_in_chunk], idx| {
                let i = chunk_origin[0] + i_in_chunk;
                let j = chunk_origin[1] + j_in_chunk;
                let k = chunk_origin[2] + k_in_chunk;

                let voxel = if i >= self.grid_shape[0]
                    || j >= self.grid_shape[1]
                    || k >= self.grid_shape[2]
                {
                    Voxel::maximally_outside()
                } else {
                    let signed_distance = VoxelSignedDistance::from_f32(signed_distances[idx]);

                    if signed_distance.is_negative() {
                        chunk_is_empty = false;
                        Voxel::non_empty(VoxelType::dummy(), signed_distance)
                    } else {
                        Voxel::empty(signed_distance)
                    }
                };

                voxels.push(voxel);
            },
        );

        if !chunk_is_empty {
            self.voxel_type_generator.set_voxel_types_for_chunk(
                &mut voxels[start_voxel_idx..],
                &mut buffers.voxel_type,
                &chunk_origin_in_root_space,
            );
        }
    }
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::{
        generation::{
            sdf::{
                BoxSDFGenerator, GradientNoiseSDFGenerator, SDFGeneratorNode, SphereSDFGenerator,
            },
            voxel_type::{GradientNoiseVoxelTypeGenerator, SameVoxelTypeGenerator},
        },
        voxel_types::VoxelTypeRegistry,
    };
    use allocator_api2::alloc::Global;
    use arbitrary::{Arbitrary, MaxRecursionReached, Result, Unstructured, size_hint};
    use std::mem;

    const MAX_SIZE: usize = 200;

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    enum ArbitrarySDFGeneratorNode {
        Box(BoxSDFGenerator),
        Sphere(SphereSDFGenerator),
        GradientNoise(GradientNoiseSDFGenerator),
    }

    impl<'a> Arbitrary<'a> for SDFVoxelGenerator {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_extent = 10.0 * arbitrary_norm_f64(u)?.max(1e-6);
            let sdf_generator = u.arbitrary()?;
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
                    SDFGenerator::size_hint(depth),
                    VoxelTypeGenerator::size_hint(depth),
                ]))
            })
        }
    }

    impl Arbitrary<'_> for SDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let primitive = match u.arbitrary()? {
                ArbitrarySDFGeneratorNode::Box(generator) => SDFGeneratorNode::Box(generator),
                ArbitrarySDFGeneratorNode::Sphere(generator) => SDFGeneratorNode::Sphere(generator),
                ArbitrarySDFGeneratorNode::GradientNoise(generator) => {
                    SDFGeneratorNode::GradientNoise(generator)
                }
            };
            let mut nodes = AVec::new();
            nodes.push(primitive);
            Ok(Self::new(Global, nodes, 0).unwrap())
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            ArbitrarySDFGeneratorNode::size_hint(depth)
        }
    }

    impl Arbitrary<'_> for BoxSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let extent_x =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_y =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_z =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            Ok(Self::new([extent_x, extent_y, extent_z]))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 6 * mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for SphereSDFGenerator {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let radius = u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE / 2 - 1) as f32
                + arbitrary_norm_f32(u)?;
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
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_y =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let extent_z =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            let noise_frequency = 0.15 * arbitrary_norm_f32(u)?;
            let noise_threshold = arbitrary_norm_f32(u)?;
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
            let noise_frequency = 0.15 * arbitrary_norm_f32(u)?;
            let voxel_type_frequency = 0.15 * arbitrary_norm_f32(u)?;
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

    fn arbitrary_norm_f32(u: &mut Unstructured<'_>) -> Result<f32> {
        arbitrary_norm_f64(u).map(|value| value as f32)
    }
}
