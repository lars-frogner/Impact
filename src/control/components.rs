//! [`Component`](impact_ecs::component::Component)s related to user control.

use super::{motion, orientation};
use crate::component::ComponentRegistry;
use anyhow::Result;

/// Registers all control [`Component`](impact_ecs::component::Component)s.
pub fn register_control_components(registry: &mut ComponentRegistry) -> Result<()> {
    motion::components::register_motion_control_components(registry)?;
    orientation::components::register_orientation_control_components(registry)
}
