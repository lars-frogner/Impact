//! [`Component`](impact_ecs::component::Component)s related to analytical motion.

use super::{circular, constant_acceleration, constant_rotation, harmonic_oscillation, orbit};
use crate::component::ComponentRegistry;
use anyhow::Result;

/// Registers all analytical motion
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_analytical_motion_components(registry: &mut ComponentRegistry) -> Result<()> {
    circular::components::register_circular_motion_components(registry)?;
    constant_acceleration::components::register_constant_acceleration_motion_components(registry)?;
    constant_rotation::components::register_constant_rotation_motion_components(registry)?;
    harmonic_oscillation::components::register_harmonic_oscillation_motion_components(registry)?;
    orbit::components::register_orbital_motion_components(registry)
}
