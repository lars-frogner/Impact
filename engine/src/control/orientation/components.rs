//! [`Component`](impact_ecs::component::Component)s related to orientation
//! control.

use crate::physics::motion::AngularVelocity;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use roc_codegen::roc;

/// [`Component`](impact_ecs::component::Component) for entities whose
/// orientation that can be controlled by a user.
#[roc(prefix = "Comp", name = "OrientationControl")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OrientationControlComp {
    control_angular_velocity: AngularVelocity,
}

#[roc]
impl OrientationControlComp {
    /// Creates a new component for orientation control.
    #[roc(body = "{ control_angular_velocity: AngularVelocity.zero({}) }")]
    pub fn new() -> Self {
        Self {
            control_angular_velocity: AngularVelocity::zero(),
        }
    }

    /// Takes a new world angular velocity due to the controller and applies it
    /// to the given total world angular velocity.
    pub fn apply_new_control_angular_velocity(
        &mut self,
        new_control_angular_velocity: AngularVelocity,
        angular_velocity: &mut AngularVelocity,
    ) {
        *angular_velocity -= self.control_angular_velocity;
        *angular_velocity += new_control_angular_velocity;
        self.control_angular_velocity = new_control_angular_velocity;
    }
}

impl Default for OrientationControlComp {
    fn default() -> Self {
        Self::new()
    }
}
