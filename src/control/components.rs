//! [`Component`](impact_ecs::component::Component)s related to user control.

use crate::physics::{AngularVelocity, Velocity};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities whose motion
/// that can be controlled by a user.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MotionControlComp {
    control_velocity: Velocity,
}

/// [`Component`](impact_ecs::component::Component) for entities whose
/// orientation that can be controlled by a user.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OrientationControlComp {
    control_angular_velocity: AngularVelocity,
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

impl OrientationControlComp {
    /// Creates a new component for orientation control.
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
