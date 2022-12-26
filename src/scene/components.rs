//! [`Component`](impact_ecs::component::Component)s related to renderable scenes.

use crate::scene::{CameraID, MeshID, SceneGraphNodeID};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`Camera`](crate::geometry::Camera).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct CameraComp {
    /// The ID of the entity's [`Camera`](crate::geometry::Camera).
    pub id: CameraID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`Mesh`](crate::geometry::Mesh).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MeshComp {
    /// The ID of the entity's [`Mesh`](crate::geometry::Mesh).
    pub id: MeshID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a node in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphNodeComp<ID: SceneGraphNodeID + Pod> {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub id: ID,
}

impl CameraComp {
    /// Creates a new component representing a [`Camera`](crate::geometry::Camera)
    /// with the given ID.
    pub fn new(camera_id: CameraID) -> Self {
        Self { id: camera_id }
    }
}

impl MeshComp {
    /// Creates a new component representing a [`Mesh`](crate::geometry::Mesh)
    /// with the given ID.
    pub fn new(mesh_id: MeshID) -> Self {
        Self { id: mesh_id }
    }
}

impl<ID: SceneGraphNodeID + Pod> SceneGraphNodeComp<ID> {
    /// Creates a new component representing a [`SceneGraph`](crate::scene::SceneGraph)
    /// node with the given ID.
    pub fn new(node_id: ID) -> Self {
        Self { id: node_id }
    }
}
