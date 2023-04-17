//! [`Component`](impact_ecs::component::Component)s related to renderable scenes.

use crate::{
    rendering::fre,
    scene::{CameraNodeID, GroupNodeID, ModelInstanceNodeID, SceneGraphNodeID},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{world::Entity, Component};

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a scaling factor.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ScalingComp(pub fre);

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// parent entity.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ParentComp {
    pub entity: Entity,
}

/// Marker [`Component`](impact_ecs::component::Component) for entities
/// representing a group node in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphGroup;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// parent group node in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphParentNodeComp {
    pub id: GroupNodeID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a node in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphNodeComp<ID: SceneGraphNodeID> {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub id: ID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// group node in the [`SceneGraph`](crate::scene::SceneGraph).
pub type SceneGraphGroupNodeComp = SceneGraphNodeComp<GroupNodeID>;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// camera node in the [`SceneGraph`](crate::scene::SceneGraph).
pub type SceneGraphCameraNodeComp = SceneGraphNodeComp<CameraNodeID>;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// model instance node in the [`SceneGraph`](crate::scene::SceneGraph).
pub type SceneGraphModelInstanceNodeComp = SceneGraphNodeComp<ModelInstanceNodeID>;

impl ParentComp {
    /// Creates a new component representing a direct child of the given
    /// [`Entity`].
    pub fn new(parent: Entity) -> Self {
        Self { entity: parent }
    }
}

impl SceneGraphParentNodeComp {
    /// Creates a new component representing the parent
    /// [`SceneGraph`](crate::scene::SceneGraph) group node with the given ID.
    pub fn new(parent_node_id: GroupNodeID) -> Self {
        Self { id: parent_node_id }
    }
}

impl<ID: SceneGraphNodeID + Pod> SceneGraphNodeComp<ID> {
    /// Creates a new component representing a [`SceneGraph`](crate::scene::SceneGraph)
    /// node with the given ID.
    pub fn new(node_id: ID) -> Self {
        Self { id: node_id }
    }
}
