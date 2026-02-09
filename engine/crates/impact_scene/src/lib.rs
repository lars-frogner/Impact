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
use impact_id::EntityID;
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
    #[roc(parents="Comp", category="bitflags", flags=[IS_DISABLED=0, CASTS_NO_SHADOWS=1])]
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
    /// A parent entity.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ParentEntity(pub EntityID);
}

define_component_type! {
    /// Marks that an entity can have children, meaning it has a group node in
    /// the scene graph identified by the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct CanBeParent;
}

define_component_type! {
    /// A maximum distance from an anchor entity that will remove this entity
    /// when exceeded.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct RemovalBeyondDistance {
        /// The ID of the entity the distance is measured from.
        pub anchor_id: EntityID,
        /// The square of the maximum distance.
        pub max_dist_squared: f64
    }
}

#[cfg(feature = "ecs")]
impact_ecs::declare_component_flags! {
    SceneEntityFlags => impact_ecs::component::ComponentFlags::INHERITABLE,
    ParentEntity => impact_ecs::component::ComponentFlags::INHERITABLE,
    RemovalBeyondDistance => impact_ecs::component::ComponentFlags::INHERITABLE,
}

impl SceneEntityFlags {
    /// Whether the [`SceneEntityFlags::IS_DISABLED`] flag is set.
    pub fn is_disabled(&self) -> bool {
        self.contains(Self::IS_DISABLED)
    }
}

#[roc]
impl RemovalBeyondDistance {
    /// Creates a new removal beyond distance rule with the given anchor entity
    /// and distance.
    #[roc(body = "{ anchor_id, max_dist_squared: Num.to_f64(max_distance * max_distance) }")]
    pub fn new(anchor_id: EntityID, max_distance: f32) -> Self {
        Self {
            anchor_id,
            max_dist_squared: f64::from(max_distance.powi(2)),
        }
    }

    pub fn max_dist_squared(&self) -> f32 {
        self.max_dist_squared as f32
    }
}
