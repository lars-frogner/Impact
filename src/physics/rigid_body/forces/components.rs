//! [`Component`](impact_ecs::component::Component)s related to forces and
//! torques on rigid bodies.

use super::{detailed_drag, spring, uniform_gravity};
use crate::component::ComponentRegistry;
use anyhow::Result;

/// Registers all rigid body force
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_rigid_body_force_components(registry: &mut ComponentRegistry) -> Result<()> {
    uniform_gravity::components::register_uniform_gravity_force_components(registry)?;
    spring::components::register_spring_force_components(registry)?;
    detailed_drag::components::register_detailed_drag_force_components(registry)
}
