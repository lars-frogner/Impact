//! [`Component`](impact_ecs::component::Component)s related to motion with
//! constant acceleration.

use crate::physics::{
    fph,
    motion::{Acceleration, Position, Velocity},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use roc_codegen::roc;

/// [`Component`](impact_ecs::component::Component) for entities that follow a
/// fixed trajectory over time governed by a constant acceleration vector.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
/// [`VelocityComp`](crate::physics::VelocityComp).
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ConstantAccelerationTrajectoryComp {
    /// When (in simulation time) the entity should be at the initial position.
    pub initial_time: fph,
    /// The position of the entity at the initial time.
    pub initial_position: Position,
    /// The velocity of the entity at the initial time.
    pub initial_velocity: Velocity,
    /// The constant acceleration of the entity.
    pub acceleration: Acceleration,
}

impl ConstantAccelerationTrajectoryComp {
    /// Creates a new component for a constant acceleration trajectory with the
    /// given properties.
    pub fn new(
        initial_time: fph,
        initial_position: Position,
        initial_velocity: Velocity,
        acceleration: Acceleration,
    ) -> Self {
        Self {
            initial_time,
            initial_position,
            initial_velocity,
            acceleration,
        }
    }

    /// Creates a new component for a constant velocity trajectory (no
    /// acceleration) with the given properties.
    pub fn with_constant_velocity(
        initial_time: fph,
        initial_position: Position,
        velocity: Velocity,
    ) -> Self {
        Self::new(
            initial_time,
            initial_position,
            velocity,
            Acceleration::zeros(),
        )
    }
}
