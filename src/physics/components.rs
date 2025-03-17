//! [`Component`](impact_ecs::component::Component)s related to physics.

use super::{collision, material, motion, rigid_body};
use crate::component::ComponentRegistry;
use anyhow::Result;

/// Registers all physics [`Component`](impact_ecs::component::Component)s.
pub fn register_physics_components(registry: &mut ComponentRegistry) -> Result<()> {
    material::components::register_physics_material_components(registry)?;
    motion::components::register_motion_components(registry)?;
    rigid_body::components::register_rigid_body_components(registry)?;
    collision::components::register_collision_components(registry)
}
