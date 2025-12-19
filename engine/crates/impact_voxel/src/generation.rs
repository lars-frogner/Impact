//! Generation of spatial voxel distributions.

pub mod import;
pub mod sdf;
pub mod voxel_type;

use crate::{
    Voxel, VoxelSignedDistance,
    chunks::{ChunkedVoxelObject, LoopForChunkVoxels},
    generation::sdf::meta::MetaSDFGraph,
    voxel_types::VoxelType,
};
use impact_alloc::{Allocator, Global};
use impact_geometry::AxisAlignedBox;
use impact_math::{hash64, stringhash64_newtype};
use impact_resource::{Resource, ResourceID, registry::ImmutableResourceRegistry};
use nalgebra::{Point3, Vector3};
use roc_integration::roc;
use sdf::{SDFGenerator, SDFGeneratorChunkBuffers};
use voxel_type::{VoxelTypeGenerator, VoxelTypeGeneratorChunkBuffers};

pub type VoxelGeneratorRegistry = ImmutableResourceRegistry<VoxelGenerator>;

stringhash64_newtype!(
    /// Identifier for a voxel generator.
    #[roc(parents = "Voxel")]
    [pub] VoxelGeneratorID
);

#[derive(Clone, Debug)]
pub struct VoxelGenerator<A: Allocator = Global> {
    pub sdf_graph: MetaSDFGraph<A>,
}

#[derive(Clone, Debug)]
pub struct VoxelGeneratorRef<'a, A: Allocator> {
    pub sdf_graph: &'a MetaSDFGraph<A>,
}

/// Represents a voxel generator that provides voxels for a chunked voxel
/// object.
pub trait ChunkedVoxelGenerator {
    type ChunkGenerationBuffers<AB: Allocator>;

    /// Returns the extent of single voxel.
    fn voxel_extent(&self) -> f32;

    /// Returns the number of voxels along the x-, y- and z-axis of the grid,
    /// respectively.
    fn grid_shape(&self) -> [usize; 3];

    /// The number of bytes that will be allocated by `create_buffers_in`.
    fn total_buffer_size(&self) -> usize;

    /// Creates temporary buffers used when generating chunks of voxels. They
    /// are meant to be reused across generation calls.
    fn create_buffers_in<AB: Allocator>(&self, alloc: AB) -> Self::ChunkGenerationBuffers<AB>;

    /// Generates voxels for a single chunk with the given chunk origin (global
    /// voxel object indices of the lower chunk corner) and writes them into the
    /// given slice.
    fn generate_chunk<AB: Allocator>(
        &self,
        buffers: &mut Self::ChunkGenerationBuffers<AB>,
        voxels: &mut [Voxel],
        chunk_origin: &[usize; 3],
    );
}

/// Generator for a voxel object from a signed distance field.
#[derive(Clone, Debug)]
pub struct SDFVoxelGenerator<A: Allocator> {
    voxel_extent: f32,
    grid_shape: [usize; 3],
    shifted_grid_center: Point3<f32>,
    sdf_generator: SDFGenerator<A>,
    voxel_type_generator: VoxelTypeGenerator,
}

#[derive(Clone, Debug)]
pub struct SDFVoxelGeneratorChunkBuffers<A: Allocator> {
    sdf: SDFGeneratorChunkBuffers<A>,
    voxel_type: VoxelTypeGeneratorChunkBuffers<A>,
}

impl ResourceID for VoxelGeneratorID {}

#[roc(dependencies = [impact_math::hash::Hash64])]
impl VoxelGeneratorID {
    #[roc(body = "Hashing.hash_str_64(name)")]
    /// Creates a voxel generator ID hashed from the given name.
    pub fn from_name(name: &str) -> Self {
        Self(hash64!(name))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for VoxelGeneratorID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for VoxelGeneratorID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_name(&s))
    }
}

impl Resource for VoxelGenerator {
    type ID = VoxelGeneratorID;
}

#[cfg(feature = "serde")]
impl<A: Allocator> serde::Serialize for VoxelGenerator<A> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("VoxelGenerator", 1)?;
        s.serialize_field("sdf_graph", &self.sdf_graph)?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, A> serde::Deserialize<'de> for VoxelGenerator<A>
where
    A: Allocator + Default,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::{fmt, marker::PhantomData};

        struct VoxelGeneratorVisitor<A>(PhantomData<A>);

        impl<'de, A> Visitor<'de> for VoxelGeneratorVisitor<A>
        where
            A: Allocator + Default,
        {
            type Value = VoxelGenerator<A>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("struct VoxelGenerator")
            }

            fn visit_map<V>(self, mut map: V) -> Result<VoxelGenerator<A>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut sdf_graph = None;
                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "sdf_graph" => {
                            if sdf_graph.is_some() {
                                return Err(de::Error::duplicate_field("sdf_graph"));
                            }
                            sdf_graph = Some(map.next_value()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                let sdf_graph = sdf_graph.ok_or_else(|| de::Error::missing_field("sdf_graph"))?;
                Ok(VoxelGenerator { sdf_graph })
            }
        }

        deserializer.deserialize_struct(
            "VoxelGenerator",
            &["sdf_graph"],
            VoxelGeneratorVisitor(PhantomData),
        )
    }
}

#[cfg(feature = "serde")]
impl<'a, A: Allocator> serde::Serialize for VoxelGeneratorRef<'a, A> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("VoxelGeneratorRef", 1)?;
        s.serialize_field("sdf_graph", self.sdf_graph)?;
        s.end()
    }
}

impl<A: Allocator> SDFVoxelGenerator<A> {
    /// Creates a new voxel generator using the given signed distance field
    /// and voxel type generators.
    pub fn new(
        voxel_extent: f32,
        sdf_generator: SDFGenerator<A>,
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

impl<A: Allocator> ChunkedVoxelGenerator for SDFVoxelGenerator<A> {
    type ChunkGenerationBuffers<AB: Allocator> = SDFVoxelGeneratorChunkBuffers<AB>;

    #[inline]
    fn voxel_extent(&self) -> f32 {
        self.voxel_extent
    }

    #[inline]
    fn grid_shape(&self) -> [usize; 3] {
        self.grid_shape
    }

    #[inline]
    fn total_buffer_size(&self) -> usize {
        self.sdf_generator.total_buffer_size_for_chunk()
            + self.voxel_type_generator.total_buffer_size()
    }

    fn create_buffers_in<AB: Allocator>(&self, alloc: AB) -> Self::ChunkGenerationBuffers<AB> {
        SDFVoxelGeneratorChunkBuffers {
            sdf: self.sdf_generator.create_buffers_for_chunk_in(alloc),
            voxel_type: self.voxel_type_generator.create_buffers_in(alloc),
        }
    }

    fn generate_chunk<AB: Allocator>(
        &self,
        buffers: &mut Self::ChunkGenerationBuffers<AB>,
        voxels: &mut [Voxel],
        chunk_origin: &[usize; 3],
    ) {
        assert_eq!(voxels.len(), ChunkedVoxelObject::chunk_voxel_count());

        if self.sdf_generator.is_empty()
            || chunk_origin
                .iter()
                .zip(self.grid_shape)
                .any(|(&origin, size)| origin >= size)
        {
            voxels.fill(Voxel::maximally_outside());
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

        let mut chunk_is_empty = true;

        LoopForChunkVoxels::over_all().execute_with_linear_idx(
            &mut |&[i_in_chunk, j_in_chunk, k_in_chunk], idx| {
                let i = chunk_origin[0] + i_in_chunk;
                let j = chunk_origin[1] + j_in_chunk;
                let k = chunk_origin[2] + k_in_chunk;

                voxels[idx] = if i >= self.grid_shape[0]
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
            },
        );

        if !chunk_is_empty {
            self.voxel_type_generator.set_voxel_types_for_chunk(
                voxels,
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
            sdf::{BoxSDF, CapsuleSDF, SDFNode, SphereSDF},
            voxel_type::{GradientNoiseVoxelTypeGenerator, SameVoxelTypeGenerator},
        },
        voxel_types::VoxelTypeRegistry,
    };
    use arbitrary::{Arbitrary, MaxRecursionReached, Result, Unstructured, size_hint};
    use impact_alloc::Global;
    use std::mem;

    const MAX_SIZE: usize = 200;

    #[allow(clippy::large_enum_variant)]
    #[derive(Clone, Debug, Arbitrary)]
    enum ArbitrarySDFGeneratorNode {
        Sphere(SphereSDF),
        Capsule(CapsuleSDF),
        Box(BoxSDF),
    }

    impl<'a> Arbitrary<'a> for SDFVoxelGenerator<Global> {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let voxel_extent = 10.0 * arbitrary_norm_f32(u)?.max(1e-6);
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

    impl Arbitrary<'_> for SDFGenerator<Global> {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let primitive = match u.arbitrary()? {
                ArbitrarySDFGeneratorNode::Sphere(generator) => SDFNode::Sphere(generator),
                ArbitrarySDFGeneratorNode::Capsule(generator) => SDFNode::Capsule(generator),
                ArbitrarySDFGeneratorNode::Box(generator) => SDFNode::Box(generator),
            };
            Ok(Self::new_in(Global, &[primitive], 0).unwrap())
        }

        fn size_hint(depth: usize) -> (usize, Option<usize>) {
            ArbitrarySDFGeneratorNode::size_hint(depth)
        }
    }

    impl Arbitrary<'_> for SphereSDF {
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

    impl Arbitrary<'_> for CapsuleSDF {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let segment_length = u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE / 2 - 1) as f32
                + arbitrary_norm_f32(u)?;
            let radius =
                u.arbitrary_len::<usize>()?.clamp(1, MAX_SIZE - 1) as f32 + arbitrary_norm_f32(u)?;
            Ok(Self::new(segment_length, radius))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 4 * mem::size_of::<usize>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for BoxSDF {
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
