//! [`Component`](impact_ecs::component::Component)s related to motion control.

use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use impact_physics::quantities::Velocity;
use roc_integration::roc;

/// [`Component`](impact_ecs::component::Component) for entities whose motion
/// that can be controlled by a user.
#[roc(parents = "Comp", name = "MotionControl")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MotionControlComp {
    control_velocity: Velocity,
}

#[roc]
impl MotionControlComp {
    /// Creates a new component for motion control.
    #[roc(body = "{ control_velocity: Vector3.zero }")]
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
