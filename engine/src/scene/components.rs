//! [`Component`](impact_ecs::component::Component)s related to scenes.

use crate::scene::{CameraNodeID, GroupNodeID, ModelInstanceNodeID, SceneEntityFlags};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent, world::Entity};
use roc_codegen::roc;

/// [`Component`](impact_ecs::component::Component) for entities that
/// participate in a scene and have associated [`SceneEntityFlags`].
///
/// If not specified, this component is automatically added to any new entity
/// that has a model, light or rigid body.
#[roc(parents = "Comp", name = "SceneEntityFlags")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct SceneEntityFlagsComp(pub SceneEntityFlags);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a parent entity.
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphParentNodeComp`] for the entity. It is therefore not kept after
/// entity creation.
#[roc(parents = "Comp", name = "Parent")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct ParentComp {
    pub entity: Entity,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities representing a group node in the
/// [`SceneGraph`](crate::scene::SceneGraph).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphGroupNodeComp`] for the entity. It is therefore not kept after
/// entity creation.
#[roc(parents = "Comp", name = "SceneGraphGroup")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct SceneGraphGroupComp;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that should never be frustum culled in the
/// [`SceneGraph`](crate::scene::SceneGraph).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphModelInstanceNodeComp`] for the entity. It is therefore not kept
/// after entity creation.
#[roc(parents = "Comp", name = "Uncullable")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct UncullableComp;

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// parent group node in the [`SceneGraph`](crate::scene::SceneGraph).
#[roc(parents = "Comp", name = "SceneGraphParentNode")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphParentNodeComp {
    pub id: GroupNodeID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// group node in the [`SceneGraph`](crate::scene::SceneGraph).
#[roc(parents = "Comp", name = "SceneGraphGroupNode")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphGroupNodeComp {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub id: GroupNodeID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// camera node in the [`SceneGraph`](crate::scene::SceneGraph).
#[roc(parents = "Comp", name = "SceneGraphCameraNode")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphCameraNodeComp {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub id: CameraNodeID,
}

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// model instance node in the [`SceneGraph`](crate::scene::SceneGraph).
#[roc(parents = "Comp", name = "SceneGraphModelInstanceNode")]
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SceneGraphModelInstanceNodeComp {
    /// The ID of the [`SceneGraph`](crate::scene::SceneGraph) node
    /// representing the entity.
    pub id: ModelInstanceNodeID,
}

impl SceneEntityFlagsComp {
    /// Whether the [`SceneEntityFlags::IS_DISABLED`] flag is set.
    pub fn is_disabled(&self) -> bool {
        self.0.contains(SceneEntityFlags::IS_DISABLED)
    }
}

#[roc]
impl ParentComp {
    /// Creates a new component representing a direct child of the given
    /// [`Entity`].
    #[roc(body = "{ entity: parent }")]
    pub fn new(parent: Entity) -> Self {
        Self { entity: parent }
    }
}

#[roc]
impl SceneGraphParentNodeComp {
    /// Creates a new component representing the parent
    /// [`SceneGraph`](crate::scene::SceneGraph) group node with the given ID.
    #[roc(body = "{ id: parent_node_id }")]
    pub fn new(parent_node_id: GroupNodeID) -> Self {
        Self { id: parent_node_id }
    }
}

#[roc]
impl SceneGraphGroupNodeComp {
    /// Creates a new component representing a
    /// [`SceneGraph`](crate::scene::SceneGraph) group node with the given ID.
    #[roc(body = "{ id: node_id }")]
    pub fn new(node_id: GroupNodeID) -> Self {
        Self { id: node_id }
    }
}

#[roc]
impl SceneGraphCameraNodeComp {
    /// Creates a new component representing a
    /// [`SceneGraph`](crate::scene::SceneGraph) camera node with the given ID.
    #[roc(body = "{ id: node_id }")]
    pub fn new(node_id: CameraNodeID) -> Self {
        Self { id: node_id }
    }
}

#[roc]
impl SceneGraphModelInstanceNodeComp {
    /// Creates a new component representing a
    /// [`SceneGraph`](crate::scene::SceneGraph) model instance node with the
    /// given ID.
    #[roc(body = "{ id: node_id }")]
    pub fn new(node_id: ModelInstanceNodeID) -> Self {
        Self { id: node_id }
    }
}
