//! Resources management for scenes.

pub mod camera;
pub mod components;
pub mod entity;
pub mod graph;
pub mod light;
pub mod model;
pub mod skybox;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

bitflags! {
    /// Bitflags encoding a set of binary states or properties for an entity in
    /// a scene.
    #[roc(parents="Scene", category="primitive")] // <- Not auto-generated, so keep Roc code synced
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct SceneEntityFlags: u8 {
        /// The entity should not affect the scene in any way.
        const IS_DISABLED      = 1 << 0;
        /// The entity should not participate in shadow maps.
        const CASTS_NO_SHADOWS = 1 << 1;
    }
}
