//! [`Component`](impact_ecs::component::Component)s related to renderable scenes.

use crate::scene::SceneGraphNodeID;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that can
/// be rendered, and thus have a node in the [`SceneGraph`](crate::scene::SceneGraph).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct Renderable<ID: SceneGraphNodeID + Pod> {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub node_id: ID,
}
