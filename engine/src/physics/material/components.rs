//! [`Component`](impact_ecs::component::Component)s related to physics
//! materials.

use super::ContactResponseParameters;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use roc_integration::roc;

/// [`Component`](impact_ecs::component::Component) for entities whose
/// [`ContactResponseParameters`] are the same across their surface.
#[roc(parents = "Comp", name = "UniformContactResponse")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct UniformContactResponseComp(pub ContactResponseParameters);
