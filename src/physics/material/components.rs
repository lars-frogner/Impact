//! [`Component`](impact_ecs::component::Component)s related to physics
//! materials.

use super::ContactResponseParameters;
use crate::component::ComponentRegistry;
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities whose
/// [`ContactResponseParameters`] are the same across their surface.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Zeroable, Pod, Component)]
pub struct UniformContactResponseComp(pub ContactResponseParameters);

/// Registers all physics material [`Component`](impact_ecs::component::Component)s.
pub fn register_physics_material_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, UniformContactResponseComp)
}
