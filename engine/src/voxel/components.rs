//! [`Component`](impact_ecs::component::Component)s related to voxels.

use crate::voxel::{
    VoxelObjectID,
    voxel_types::{VoxelType, VoxelTypeRegistry},
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent};
use impact_geometry::{Capsule, Sphere};
use impact_math::Hash32;
use nalgebra::{Point3, Vector3};
use roc_integration::roc;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose voxel type is the same everywhere.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "SameVoxelType")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct SameVoxelTypeComp {
    /// The index of the voxel type.
    voxel_type_idx: usize,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose voxel types are distributed according to a gradient noise
/// pattern.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "GradientNoiseVoxelTypes")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct GradientNoiseVoxelTypesComp {
    n_voxel_types: usize,
    voxel_type_name_hashes: [Hash32; GradientNoiseVoxelTypesComp::VOXEL_TYPE_ARRAY_SIZE],
    noise_frequency: f64,
    voxel_type_frequency: f64,
    pub seed: u64,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose voxel signed distance field should be modified by unions
/// with multiscale sphere grid (<https://iquilezles.org/articles/fbmsdf>/).
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "MultiscaleSphereModification")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct MultiscaleSphereModificationComp {
    pub octaves: usize,
    pub max_scale: f64,
    pub persistence: f64,
    pub inflation: f64,
    pub smoothness: f64,
    pub seed: u64,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose voxel signed distance field should be perturbed by
/// multifractal noise.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "MultifractalNoiseModification")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct MultifractalNoiseModificationComp {
    pub octaves: usize,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
    pub amplitude: f64,
    pub seed: u64,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities comprised of voxels in a box configuration.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "VoxelBox")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct VoxelBoxComp {
    /// The extent of a single voxel.
    pub voxel_extent: f64,
    /// The number of voxels along the box in the x-direction.
    pub extent_x: f64,
    /// The number of voxels along the box in the y-direction.
    pub extent_y: f64,
    /// The number of voxels along the box in the z-direction.
    pub extent_z: f64,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities comprised of voxels in a spherical configuration.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "VoxelSphere")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct VoxelSphereComp {
    /// The extent of a single voxel.
    pub voxel_extent: f64,
    /// The number of voxels along the radius of the sphere.
    pub radius: f64,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities comprised of voxels in a configuration described by the smooth
/// union of two spheres.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "VoxelSphereUnion")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct VoxelSphereUnionComp {
    /// The extent of a single voxel.
    pub voxel_extent: f64,
    /// The number of voxels along the radius of the first sphere.
    pub radius_1: f64,
    /// The number of voxels along the radius of the second sphere.
    pub radius_2: f64,
    /// The offset in number of voxels in each dimension between the centers of
    /// the two spheres.
    pub center_offsets: Vector3<f64>,
    /// The smoothness of the union operation.
    pub smoothness: f64,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities comprised of voxels in a gradient noise pattern.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "VoxelGradientNoisePattern")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct VoxelGradientNoisePatternComp {
    /// The extent of a single voxel.
    pub voxel_extent: f64,
    /// The maximum number of voxels in the x-direction.
    pub extent_x: f64,
    /// The maximum number of voxels in the y-direction.
    pub extent_y: f64,
    /// The maximum number of voxels in the z-direction.
    pub extent_z: f64,
    /// The spatial frequency of the noise pattern.
    pub noise_frequency: f64,
    /// The threshold noise value for generating a voxel.
    pub noise_threshold: f64,
    /// The seed for the noise pattern.
    pub seed: u64,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// [`ChunkedVoxelObject`](crate::voxel::ChunkedVoxelObject).
#[roc(parents = "Comp", name = "VoxelObject")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelObjectComp {
    /// The ID of the entity's
    /// [`ChunkedVoxelObject`](crate::voxel::ChunkedVoxelObject).
    pub voxel_object_id: VoxelObjectID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// sphere that absorbs voxels it comes in contact with. The rate of absorption
/// is highest at the center of the sphere and decreases quadratically to zero
/// at the full radius.
///
/// Does nothing if the entity does not have a
/// [`impact_geometry::ReferenceFrame`].
#[roc(parents = "Comp", name = "VoxelAbsorbingSphere")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelAbsorbingSphereComp {
    /// The offset of the sphere in the reference frame of the entity.
    offset: Vector3<f64>,
    /// The radius of the sphere.
    radius: f64,
    /// The maximum rate of absorption (at the center of the sphere).
    rate: f64,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// capsule that absorbs voxels it comes in contact with. The rate of absorption
/// is highest at the central line segment of the capsule and decreases
/// quadratically to zero at the capsule boundary.
///
/// Does nothing if the entity does not have a
/// [`impact_geometry::ReferenceFrame`].
#[roc(parents = "Comp", name = "VoxelAbsorbingCapsule")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelAbsorbingCapsuleComp {
    /// The offset of the starting point of the capsule's central line segment
    /// in the reference frame of the entity.
    offset_to_segment_start: Vector3<f64>,
    /// The displacement vector from the start to the end of the capsule's
    /// central line segment in the reference frame of the entity.
    segment_vector: Vector3<f64>,
    /// The radius of the capsule.
    radius: f64,
    /// The maximum rate of absorption (at the central line segment of the
    /// capsule).
    rate: f64,
}

#[roc(dependencies=[VoxelType])]
impl SameVoxelTypeComp {
    /// Creates a new component for an entity comprised of voxels of the given
    /// type.
    #[roc(body = "{ voxel_type_idx: NativeNum.to_usize(voxel_type) }")]
    pub fn new(voxel_type: VoxelType) -> Self {
        Self {
            voxel_type_idx: voxel_type.idx(),
        }
    }

    /// Returns the voxel type.
    pub fn voxel_type(&self) -> VoxelType {
        VoxelType::from_idx(self.voxel_type_idx)
    }
}

#[roc]
impl GradientNoiseVoxelTypesComp {
    #[roc(expr = "256")]
    const VOXEL_TYPE_ARRAY_SIZE: usize = VoxelTypeRegistry::max_n_voxel_types().next_power_of_two();

    #[roc(body = r#"
    n_voxel_types = List.len(voxel_type_names)
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect n_voxel_types > 0
    # expect n_voxel_types <= voxel_type_array_size
    unpadded_voxel_type_name_hashes = voxel_type_names |> List.map(Hashing.hash_str_32)
    padding_len = voxel_type_array_size - n_voxel_types
    voxel_type_name_hashes = List.concat(
        unpadded_voxel_type_name_hashes,
        List.repeat(Hashing.hash_str_32(""), padding_len),
    )
    {
        n_voxel_types,
        voxel_type_name_hashes,
        noise_frequency,
        voxel_type_frequency,
        seed,
    }
    "#)]
    pub fn new(
        voxel_type_names: &[&str],
        noise_frequency: f64,
        voxel_type_frequency: f64,
        seed: u64,
    ) -> Self {
        let n_voxel_types = voxel_type_names.len();
        assert!(n_voxel_types > 0);
        assert!(n_voxel_types <= VoxelTypeRegistry::max_n_voxel_types());

        let mut voxel_type_name_hashes = [Hash32::zeroed(); Self::VOXEL_TYPE_ARRAY_SIZE];
        for (idx, name) in voxel_type_names.iter().enumerate() {
            voxel_type_name_hashes[idx] = Hash32::from_str(name);
        }

        Self {
            n_voxel_types,
            voxel_type_name_hashes,
            noise_frequency,
            voxel_type_frequency,
            seed,
        }
    }

    pub fn voxel_types(&self, voxel_type_registry: &VoxelTypeRegistry) -> Result<Vec<VoxelType>> {
        let mut voxel_types = Vec::with_capacity(self.n_voxel_types);
        for (idx, &name_hash) in self.voxel_type_name_hashes[..self.n_voxel_types]
            .iter()
            .enumerate()
        {
            voxel_types.push(
                voxel_type_registry
                    .voxel_type_for_name_hash(name_hash)
                    .ok_or_else(|| anyhow!("Missing voxel type for name at index {}", idx))?,
            );
        }
        Ok(voxel_types)
    }

    pub fn noise_frequency(&self) -> f64 {
        self.noise_frequency
    }

    pub fn voxel_type_frequency(&self) -> f64 {
        self.voxel_type_frequency
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }
}

#[roc]
impl MultiscaleSphereModificationComp {
    #[roc(body = r#"
    {
        octaves,
        max_scale,
        persistence,
        inflation,
        smoothness,
        seed,
    }"#)]
    pub fn new(
        octaves: usize,
        max_scale: f64,
        persistence: f64,
        inflation: f64,
        smoothness: f64,
        seed: u64,
    ) -> Self {
        Self {
            octaves,
            max_scale,
            persistence,
            inflation,
            smoothness,
            seed,
        }
    }
}

#[roc]
impl MultifractalNoiseModificationComp {
    #[roc(body = r#"
    {
        octaves,
        frequency,
        lacunarity,
        persistence,
        amplitude,
        seed,
    }"#)]
    pub fn new(
        octaves: usize,
        frequency: f64,
        lacunarity: f64,
        persistence: f64,
        amplitude: f64,
        seed: u64,
    ) -> Self {
        Self {
            octaves,
            frequency,
            lacunarity,
            persistence,
            amplitude,
            seed,
        }
    }
}

#[roc]
impl VoxelBoxComp {
    /// Creates a new component for a box with the given voxel extent
    /// and number of voxels in each direction.
    ///
    /// # Panics
    /// - If the voxel extent is negative.
    /// - If either of the extents is zero or negative.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect extent_x >= 0.0
    # expect extent_y >= 0.0
    # expect extent_z >= 0.0
    {
        voxel_extent,
        extent_x,
        extent_y,
        extent_z,
    }"#)]
    pub fn new(voxel_extent: f64, extent_x: f64, extent_y: f64, extent_z: f64) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(extent_x >= 0.0);
        assert!(extent_y >= 0.0);
        assert!(extent_z >= 0.0);
        Self {
            voxel_extent,
            extent_x,
            extent_y,
            extent_z,
        }
    }

    pub fn extents_in_voxels(&self) -> [f64; 3] {
        [self.extent_x, self.extent_y, self.extent_z]
    }
}

#[roc]
impl VoxelSphereComp {
    /// Creates a new component for a sphere with the given voxel extent
    /// and number of voxels across its radius.
    ///
    /// # Panics
    /// - If the voxel extent is negative.
    /// - If the radius zero or negative.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect radius >= 0.0
    {
        voxel_extent,
        radius,
    }"#)]
    pub fn new(voxel_extent: f64, radius: f64) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(radius >= 0.0);
        Self {
            voxel_extent,
            radius,
        }
    }

    pub fn radius_in_voxels(&self) -> f64 {
        self.radius
    }
}

#[roc]
impl VoxelSphereUnionComp {
    /// Creates a new component for a sphere union with the given smoothness of
    /// the spheres with the given radii and center offsets (in voxels).
    ///
    /// # Panics
    /// - If the voxel extent is negative.
    /// - If either of the radii is zero or negative.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect radius_1 >= 0.0
    # expect radius_2 >= 0.0
    {
        voxel_extent,
        radius_1,
        radius_2,
        center_offsets,
        smoothness,
    }"#)]
    pub fn new(
        voxel_extent: f64,
        radius_1: f64,
        radius_2: f64,
        center_offsets: Vector3<f64>,
        smoothness: f64,
    ) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(radius_1 >= 0.0);
        assert!(radius_2 >= 0.0);
        Self {
            voxel_extent,
            radius_1,
            radius_2,
            center_offsets,
            smoothness,
        }
    }

    pub fn radius_1_in_voxels(&self) -> f64 {
        self.radius_1
    }

    pub fn radius_2_in_voxels(&self) -> f64 {
        self.radius_2
    }
}

#[roc]
impl VoxelGradientNoisePatternComp {
    /// Creates a new component for a gradient noise voxel pattern with the
    /// given maximum number of voxels in each direction, spatial noise
    /// frequency, noise threshold and seed.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect voxel_extent > 0.0
    # expect extent_x >= 0.0
    # expect extent_y >= 0.0
    # expect extent_z >= 0.0
    {
        voxel_extent,
        extent_x,
        extent_y,
        extent_z,
        noise_frequency,
        noise_threshold,
        seed,
    }"#)]
    pub fn new(
        voxel_extent: f64,
        extent_x: f64,
        extent_y: f64,
        extent_z: f64,
        noise_frequency: f64,
        noise_threshold: f64,
        seed: u64,
    ) -> Self {
        assert!(voxel_extent > 0.0);
        assert!(extent_x >= 0.0);
        assert!(extent_y >= 0.0);
        assert!(extent_z >= 0.0);
        Self {
            voxel_extent,
            extent_x,
            extent_y,
            extent_z,
            noise_frequency,
            noise_threshold,
            seed,
        }
    }

    pub fn extents_in_voxels(&self) -> [f64; 3] {
        [self.extent_x, self.extent_y, self.extent_z]
    }
}

#[roc]
impl VoxelAbsorbingSphereComp {
    /// Creates a new [`VoxelAbsorbingSphereComp`] with the given offset and
    /// radius in the reference frame of the entity and the given maximum
    /// absorption rate (at the center of the sphere).
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    # expect rate >= 0.0
    {
        offset,
        radius,
        rate,
    }"#)]
    pub fn new(offset: Vector3<f64>, radius: f64, rate: f64) -> Self {
        assert!(radius >= 0.0);
        assert!(rate >= 0.0);
        Self {
            offset,
            radius,
            rate,
        }
    }

    /// Returns the sphere in the reference frame of the entity.
    pub fn sphere(&self) -> Sphere<f64> {
        Sphere::new(Point3::from(self.offset), self.radius)
    }

    /// Returns the maximum absorption rate.
    pub fn rate(&self) -> f64 {
        self.rate
    }
}

#[roc]
impl VoxelAbsorbingCapsuleComp {
    /// Creates a new [`VoxelAbsorbingCapsuleComp`] with the given offset to the
    /// start of the capsule's central line segment, displacement from the start
    /// to the end of the line segment and radius, all in the reference frame of
    /// the entity, as well as the given maximum absorption rate (at the central
    /// line segment).
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect radius >= 0.0
    # expect rate >= 0.0
    {
        offset_to_segment_start,
        segment_vector,
        radius,
        rate,
    }"#)]
    pub fn new(
        offset_to_segment_start: Vector3<f64>,
        segment_vector: Vector3<f64>,
        radius: f64,
        rate: f64,
    ) -> Self {
        assert!(radius >= 0.0);
        assert!(rate >= 0.0);
        Self {
            offset_to_segment_start,
            segment_vector,
            radius,
            rate,
        }
    }

    /// Returns the capsule in the reference frame of the entity.
    pub fn capsule(&self) -> Capsule<f64> {
        Capsule::new(
            Point3::from(self.offset_to_segment_start),
            self.segment_vector,
            self.radius,
        )
    }

    /// Returns the maximum absorption rate.
    pub fn rate(&self) -> f64 {
        self.rate
    }
}
