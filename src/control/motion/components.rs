//! [`Component`](impact_ecs::component::Component)s related to motion control.

use crate::{component::ComponentRegistry, physics::motion::Velocity};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities whose motion
/// that can be controlled by a user.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MotionControlComp {
    control_velocity: Velocity,
}

impl MotionControlComp {
    /// Creates a new component for motion control.
    pub fn new() -> Self {
        Self {
            control_velocity: Velocity::zeros(),
        }
    }

    /// Takes a new world velocity due to the controller and applies it to the
    /// given total world velocity.
    pub fn apply_new_control_velocity(
        &mut self,
        new_control_velocity: Velocity,
        velocity: &mut Velocity,
    ) {
        *velocity -= self.control_velocity;
        *velocity += new_control_velocity;
        self.control_velocity = new_control_velocity;
    }
}

impl Default for MotionControlComp {
    fn default() -> Self {
        Self::new()
    }
}

/// Registers all motion control
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_motion_control_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, MotionControlComp)
}
