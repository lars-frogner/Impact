//! [`Component`](impact_ecs::component::Component)s related to voxels.

use crate::{
    component::ComponentRegistry,
    voxel::{
        voxel_types::{VoxelType, VoxelTypeRegistry},
        VoxelObjectID,
    },
};
use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use impact_utils::{compute_hash_str_32, Hash32};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities whose voxel type is the same everywhere.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SameVoxelTypeComp {
    /// The index of the voxel type.
    voxel_type_idx: usize,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities whose voxel types are distributed according to a gradient noise
/// pattern.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct GradientNoiseVoxelTypesComp {
    n_voxel_types: usize,
    voxel_type_name_hashes: [Hash32; GradientNoiseVoxelTypesComp::VOXEL_TYPE_ARRAY_SIZE],
    noise_frequency: f64,
    voxel_type_frequency: f64,
    pub seed: u64,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities comprised of voxels in a box configuration.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelBoxComp {
    /// The extent of a single voxel.
    pub voxel_extent: f64,
    /// The number of voxels along the box in the x-direction.
    pub size_x: usize,
    /// The number of voxels along the box in the y-direction.
    pub size_y: usize,
    /// The number of voxels along the box in the z-direction.
    pub size_z: usize,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities comprised of voxels in a spherical configuration.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelSphereComp {
    /// The extent of a single voxel.
    voxel_extent: f64,
    /// The number of voxels along the diameter of the sphere.
    n_voxels_across: usize,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities comprised of voxels in a gradient noise pattern.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelGradientNoisePatternComp {
    /// The extent of a single voxel.
    pub voxel_extent: f64,
    /// The maximum number of voxels in the x-direction.
    pub size_x: usize,
    /// The maximum number of voxels in the y-direction.
    pub size_y: usize,
    /// The maximum number of voxels in the z-direction.
    pub size_z: usize,
    /// The spatial frequency of the noise pattern.
    pub noise_frequency: f64,
    /// The threshold noise value for generating a voxel.
    pub noise_threshold: f64,
    /// The seed for the noise pattern.
    pub seed: u64,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// [`ChunkedVoxelObject`](crate::voxel::ChunkedVoxelObject).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelObjectComp {
    /// The ID of the entity's
    /// [`ChunkedVoxelObject`](crate::voxel::ChunkedVoxelObject).
    pub voxel_object_id: VoxelObjectID,
}

impl SameVoxelTypeComp {
    /// Creates a new component for an entity comprised of voxels of the given
    /// type.
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

impl GradientNoiseVoxelTypesComp {
    const VOXEL_TYPE_ARRAY_SIZE: usize = VoxelTypeRegistry::max_n_voxel_types().next_power_of_two();

    pub fn new<S: AsRef<str>>(
        voxel_type_names: impl IntoIterator<Item = S>,
        noise_frequency: f64,
        voxel_type_frequency: f64,
        seed: u64,
    ) -> Self {
        let mut n_voxel_types = 0;
        let mut voxel_type_name_hashes = [Hash32::zeroed(); Self::VOXEL_TYPE_ARRAY_SIZE];
        for name in voxel_type_names {
            assert!(n_voxel_types < VoxelTypeRegistry::max_n_voxel_types());
            voxel_type_name_hashes[n_voxel_types] = compute_hash_str_32(name.as_ref());
            n_voxel_types += 1;
        }
        assert!(n_voxel_types > 0);
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

impl VoxelBoxComp {
    /// Creates a new component for a uniform box with the given voxel extent
    /// and number of voxels in each direction.
    pub fn new(voxel_extent: f64, size_x: usize, size_y: usize, size_z: usize) -> Self {
        Self {
            voxel_extent,
            size_x,
            size_y,
            size_z,
        }
    }
}

impl VoxelSphereComp {
    /// Creates a new component for a uniform sphere with the given voxel extent
    /// and number of voxels across its diameter.
    ///
    /// # Panics
    /// If the given number of voxels across is zero.
    pub fn new(voxel_extent: f64, n_voxels_across: usize) -> Self {
        assert_ne!(n_voxels_across, 0);
        Self {
            voxel_extent,
            n_voxels_across,
        }
    }

    /// Returns the extent of a single voxel.
    pub fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }

    /// Returns the number of voxels across the sphere's diameter.
    pub fn n_voxels_across(&self) -> usize {
        self.n_voxels_across
    }
}

impl VoxelGradientNoisePatternComp {
    /// Creates a new component for a gradient noise voxel pattern with the
    /// given maximum number of voxels in each direction, spatial noise
    /// frequency, noise threshold and seed.
    pub fn new(
        voxel_extent: f64,
        size_x: usize,
        size_y: usize,
        size_z: usize,
        noise_frequency: f64,
        noise_threshold: f64,
        seed: u64,
    ) -> Self {
        Self {
            voxel_extent,
            size_x,
            size_y,
            size_z,
            noise_frequency,
            noise_threshold,
            seed,
        }
    }
}

/// Registers all voxel [`Component`](impact_ecs::component::Component)s.
pub fn register_voxel_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, SameVoxelTypeComp)?;
    register_setup_component!(registry, GradientNoiseVoxelTypesComp)?;
    register_setup_component!(registry, VoxelBoxComp)?;
    register_setup_component!(registry, VoxelSphereComp)?;
    register_setup_component!(registry, VoxelGradientNoisePatternComp)?;
    register_setup_component!(registry, VoxelObjectComp)
}
