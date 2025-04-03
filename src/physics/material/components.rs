//! [`Component`](impact_ecs::component::Component)s related to physics
//! materials.

use super::ContactResponseParameters;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities whose
/// [`ContactResponseParameters`] are the same across their surface.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct UniformContactResponseComp(pub ContactResponseParameters);
