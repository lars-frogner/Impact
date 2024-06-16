//! [`Component`](impact_ecs::component::Component)s related to scene graphs.

use crate::{
    components::ComponentRegistry,
    scene::{CameraNodeID, GroupNodeID, ModelInstanceNodeID, SceneGraphNodeID},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::{world::Entity, Component};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a parent entity.
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphParentNodeComp`] for the entity. It is therefore not kept after
/// entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ParentComp {
    pub entity: Entity,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities representing a group node in the
/// [`SceneGraph`](crate::scene::SceneGraph).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphGroupNodeComp`] for the entity. It is therefore not kept after
/// entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphGroupComp;

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that should never be frustum culled in the
/// [`SceneGraph`](crate::scene::SceneGraph).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphModelInstanceNodeComp`] for the entity. It is therefore not kept
/// after entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UncullableComp;

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

impl<ID: SceneGraphNodeID> SceneGraphNodeComp<ID> {
    /// Creates a new component representing a [`SceneGraph`](crate::scene::SceneGraph)
    /// node with the given ID.
    pub fn new(node_id: ID) -> Self {
        Self { id: node_id }
    }
}

/// Registers all scene graph [`Component`](impact_ecs::component::Component)s.
pub fn register_scene_graph_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, ParentComp)?;
    register_setup_component!(registry, SceneGraphGroupComp)?;
    register_setup_component!(registry, UncullableComp)?;
    register_component!(registry, SceneGraphParentNodeComp)?;
    register_component!(registry, SceneGraphGroupNodeComp)?;
    register_component!(registry, SceneGraphCameraNodeComp)?;
    register_component!(registry, SceneGraphModelInstanceNodeComp)
}
