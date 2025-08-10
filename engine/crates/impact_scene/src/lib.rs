//! Resources management for scenes.

#[macro_use]
mod macros;

pub mod camera;
pub mod graph;
pub mod light;
pub mod model;
pub mod setup;
pub mod skybox;

#[cfg(feature = "ecs")]
pub mod systems;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use graph::{CameraNodeID, GroupNodeID, ModelInstanceNodeID};
use roc_integration::roc;

bitflags! {
    /// Bitflags encoding a set of binary states or properties for an entity in
    /// a scene.
    #[cfg_attr(
        feature = "ecs",
        doc = concat!(
            "\n\n\
            This is an ECS [`Component`](impact_ecs::component::Component). \
            If not specified, this component is automatically added to any \
            new entity that has a model, light or rigid body."
        )
    )]
    #[roc(parents="Comp", category="primitive")] // <- Not auto-generated, so keep Roc code synced
    #[repr(transparent)]
    #[cfg_attr(feature = "ecs", derive(impact_ecs::Component))]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct SceneEntityFlags: u8 {
        /// The entity should not affect the scene in any way.
        const IS_DISABLED      = 1 << 0;
        /// The entity should not participate in shadow maps.
        const CASTS_NO_SHADOWS = 1 << 1;
    }
}

define_component_type! {
    /// Handle to a parent group node in a scene graph.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SceneGraphParentNodeHandle {
        /// The ID of the parent node in the
        /// [`SceneGraph`](crate::graph::SceneGraph).
        pub id: GroupNodeID,
    }
}

define_component_type! {
    /// Handle to a group node in a scene graph.
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SceneGraphGroupNodeHandle {
        /// The ID of the group node in the
        /// [`SceneGraph`](crate::graph::SceneGraph).
        pub id: GroupNodeID,
    }
}

define_component_type! {
    /// Handle to a camera node in a scene graph.
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SceneGraphCameraNodeHandle {
        /// The ID of the camera node in the
        /// [`SceneGraph`](crate::graph::SceneGraph).
        pub id: CameraNodeID,
    }
}

define_component_type! {
    /// Handle to a model instance node in a scene graph.
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct SceneGraphModelInstanceNodeHandle {
        /// The ID of the model instance node in the
        /// [`SceneGraph`](crate::graph::SceneGraph).
        pub id: ModelInstanceNodeID,
    }
}

impl SceneEntityFlags {
    /// Whether the [`SceneEntityFlags::IS_DISABLED`] flag is set.
    pub fn is_disabled(&self) -> bool {
        self.contains(Self::IS_DISABLED)
    }
}

#[roc]
impl SceneGraphParentNodeHandle {
    /// Creates a new handle to the parent
    /// [`SceneGraph`](crate::graph::SceneGraph) group node with the given ID.
    #[roc(body = "{ id: parent_node_id }")]
    pub fn new(parent_node_id: GroupNodeID) -> Self {
        Self { id: parent_node_id }
    }
}

#[roc]
impl SceneGraphGroupNodeHandle {
    /// Creates a new handle to the [`SceneGraph`](crate::graph::SceneGraph)
    /// group node with the given ID.
    #[roc(body = "{ id: node_id }")]
    pub fn new(node_id: GroupNodeID) -> Self {
        Self { id: node_id }
    }
}

#[roc]
impl SceneGraphCameraNodeHandle {
    /// Creates a new handle to the [`SceneGraph`](crate::graph::SceneGraph)
    /// camera node with the given ID.
    #[roc(body = "{ id: node_id }")]
    pub fn new(node_id: CameraNodeID) -> Self {
        Self { id: node_id }
    }
}

#[roc]
impl SceneGraphModelInstanceNodeHandle {
    /// Creates a new handle to the [`SceneGraph`](crate::graph::SceneGraph)
    /// model instance node with the given ID.
    #[roc(body = "{ id: node_id }")]
    pub fn new(node_id: ModelInstanceNodeID) -> Self {
        Self { id: node_id }
    }
}
