//! [`Component`](impact_ecs::component::Component)s related to renderable scenes.

use crate::{
    rendering::fre,
    scene::{CameraNodeID, SceneGraphNodeID},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a scaling factor their mesh.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ScalingComp(pub fre);

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a node in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphNodeComp<ID: SceneGraphNodeID> {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub id: ID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a camera node in the [`SceneGraph`](crate::scene::SceneGraph).
pub type SceneGraphCameraNodeComp = SceneGraphNodeComp<CameraNodeID>;

impl<ID: SceneGraphNodeID + Pod> SceneGraphNodeComp<ID> {
    /// Creates a new component representing a [`SceneGraph`](crate::scene::SceneGraph)
    /// node with the given ID.
    pub fn new(node_id: ID) -> Self {
        Self { id: node_id }
    }
}
