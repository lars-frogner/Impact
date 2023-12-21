//! [`Component`](impact_ecs::component::Component)s related to voxels.

use crate::{
    components::ComponentRegistry,
    geometry::VoxelType,
    scene::{GroupNodeID, VoxelTreeID, VoxelTreeNodeID},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use num_traits::{FromPrimitive, ToPrimitive};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities comprised of identical voxels.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelTreeNodeComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelTypeComp {
    /// The index of the voxel type.
    voxel_type_idx: usize,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities comprised of voxels in a box configuration.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelTreeNodeComp`] for the entity. It is therefore not kept after entity
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
/// [`VoxelTreeNodeComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelSphereComp {
    /// The number of voxels along the diameter of the sphere.
    n_voxels_across: usize,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities with voxels represented by a
/// [`VoxelTree`](crate::geometry::VoxelTree).
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelTreeNodeComp`] for the entity. It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelTreeComp {
    /// The ID of the entity's [`VoxelTree`](crate::geometry::VoxelTree).
    pub voxel_tree_id: VoxelTreeID,
}

/// [`Component`](impact_ecs::component::Component) for entities representing a
/// [`VoxelTreeNodeID`] in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelTreeNodeComp {
    /// The group node in the [`SceneGraph`](crate::scene::SceneGraph) holding
    /// the transform to the parent space of the tree.
    pub group_node_id: GroupNodeID,
    /// The voxel tree node in the [`SceneGraph`](crate::scene::SceneGraph)
    /// holding the transforms locating each voxel instance to render in the
    /// tree's reference frame.
    pub voxel_tree_node_id: VoxelTreeNodeID,
    /// The ID of the [`VoxelTree`](crate::geometry::VoxelTree).
    pub voxel_tree_id: VoxelTreeID,
    _pad: [u8; 4],
}

impl VoxelTypeComp {
    /// Creates a new component for an entity comprised of voxels of the given
    /// type.
    pub fn new(voxel_type: VoxelType) -> Self {
        Self {
            voxel_type_idx: voxel_type.to_usize().unwrap(),
        }
    }

    /// Returns the voxel type.
    pub fn voxel_type(&self) -> VoxelType {
        VoxelType::from_usize(self.voxel_type_idx).unwrap()
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

impl VoxelTreeNodeComp {
    pub fn new(
        voxel_tree_id: VoxelTreeID,
        group_node_id: GroupNodeID,
        voxel_tree_node_id: VoxelTreeNodeID,
    ) -> Self {
        Self {
            group_node_id,
            voxel_tree_node_id,
            voxel_tree_id,
            _pad: [0; 4],
        }
    }
}

/// Registers all voxel [`Component`](impact_ecs::component::Component)s.
pub fn register_voxel_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, VoxelTypeComp)?;
    register_setup_component!(registry, VoxelBoxComp)?;
    register_setup_component!(registry, VoxelSphereComp)?;
    register_setup_component!(registry, VoxelTreeComp)?;
    register_component!(registry, VoxelTreeNodeComp)
}
