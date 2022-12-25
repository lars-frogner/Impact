//! [`Component`](impact_ecs::component::Component)s related to renderable scenes.

use crate::scene::{MeshID, SceneGraphNodeID};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a node in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphNodeComp<ID: SceneGraphNodeID + Pod> {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub id: ID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`Mesh`](crate::geometry::Mesh).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MeshComp {
    /// The ID of the entity's [`Mesh`](crate::geometry::Mesh).
    pub id: MeshID,
}
