//! [`Component`](impact_ecs::component::Component)s related to voxels.

use crate::{
    component::ComponentRegistry,
    voxel::{VoxelObjectID, VoxelType},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use num_traits::{FromPrimitive, ToPrimitive};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities comprised of identical voxels.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelObjectComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelTypeComp {
    /// The index of the voxel type.
    voxel_type_idx: usize,
    /// The extent of a single voxel.
    voxel_extent: f64,
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

impl VoxelTypeComp {
    /// Creates a new component for an entity comprised of voxels of the given
    /// type and extent.
    pub fn new(voxel_type: VoxelType, voxel_extent: f64) -> Self {
        Self {
            voxel_type_idx: voxel_type.to_usize().unwrap(),
            voxel_extent,
        }
    }

    /// Returns the voxel type.
    pub fn voxel_type(&self) -> VoxelType {
        VoxelType::from_usize(self.voxel_type_idx).unwrap()
    }

    /// Returns the extent of a single voxel.
    pub fn voxel_extent(&self) -> f64 {
        self.voxel_extent
    }
}

impl VoxelBoxComp {
    /// Creates a new component for a uniform box with the given number of
    /// voxels in each direction.
    pub fn new(size_x: usize, size_y: usize, size_z: usize) -> Self {
        Self {
            size_x,
            size_y,
            size_z,
        }
    }
}

impl VoxelSphereComp {
    /// Creates a new component for a uniform sphere with the given number of
    /// voxels across its diameter.
    ///
    /// # Panics
    /// If the given number of voxels across is zero.
    pub fn new(n_voxels_across: usize) -> Self {
        assert_ne!(n_voxels_across, 0);
        Self { n_voxels_across }
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
        size_x: usize,
        size_y: usize,
        size_z: usize,
        noise_frequency: f64,
        noise_threshold: f64,
        seed: u64,
    ) -> Self {
        Self {
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
    register_setup_component!(registry, VoxelTypeComp)?;
    register_setup_component!(registry, VoxelBoxComp)?;
    register_setup_component!(registry, VoxelSphereComp)?;
    register_setup_component!(registry, VoxelGradientNoisePatternComp)?;
    register_setup_component!(registry, VoxelObjectComp)
}
