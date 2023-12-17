//! [`Component`](impact_ecs::component::Component)s related to voxels.

use crate::{
    components::ComponentRegistry,
    geometry::VoxelType,
    scene::{GroupNodeID, ModelInstanceClusterNodeID, VoxelTreeID},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use num_traits::{FromPrimitive, ToPrimitive};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities comprised of identical voxels.
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelInstanceClusterComp`] for the entity. It is therefore not kept after
/// entity creation.
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
/// [`VoxelInstanceClusterComp`] for the entity. It is therefore not kept after
/// entity creation.
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
/// entities with voxels represented by a
/// [`VoxelTree`](crate::geometry::VoxelTree).
///
/// The purpose of this component is to aid in constructing a
/// [`VoxelInstanceClusterComp`] for the entity. It is therefore not kept after
/// entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelTreeComp {
    /// The ID of the entity's [`VoxelTree`](crate::geometry::VoxelTree).
    pub voxel_tree_id: VoxelTreeID,
}

/// [`Component`](impact_ecs::component::Component) for entities representing a
/// cluster of multiple voxel model instances.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VoxelInstanceClusterComp {
    /// The group node in the [`SceneGraph`](crate::scene::SceneGraph) holding
    /// the transform to the parent space of the cluster.
    pub group_node_id: GroupNodeID,
    /// The model instance cluster node in the
    /// [`SceneGraph`](crate::scene::SceneGraph) holding the transforms locating
    /// each voxel instance to render in the cluster's reference frame.
    pub model_instance_cluster_node_id: ModelInstanceClusterNodeID,
    /// The ID of cluster's [`VoxelTree`](crate::geometry::VoxelTree).
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

impl VoxelInstanceClusterComp {
    pub fn new(
        voxel_tree_id: VoxelTreeID,
        group_node_id: GroupNodeID,
        model_instance_cluster_node_id: ModelInstanceClusterNodeID,
    ) -> Self {
        Self {
            group_node_id,
            model_instance_cluster_node_id,
            voxel_tree_id,
            _pad: [0; 4],
        }
    }
}

/// Registers all voxel [`Component`](impact_ecs::component::Component)s.
pub fn register_voxel_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, VoxelTypeComp)?;
    register_setup_component!(registry, VoxelBoxComp)?;
    register_setup_component!(registry, VoxelTreeComp)?;
    register_component!(registry, VoxelInstanceClusterComp)
}
