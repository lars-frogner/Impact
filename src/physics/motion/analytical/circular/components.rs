//! [`Component`](impact_ecs::component::Component)s related to circular motion.

use crate::{
    component::ComponentRegistry,
    physics::{
        fph,
        motion::{Orientation, Position},
    },
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that follow a
/// circular trajectory over time with constant speed.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
/// [`VelocityComp`](crate::physics::VelocityComp).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct CircularTrajectoryComp {
    /// When (in simulation time) the entity should be at the initial position
    /// on the circle.
    pub initial_time: fph,
    /// The orientation of the orbit. The first axis of the circle's reference
    /// frame will coincide with the direction from the center to the position
    /// of the entity at the initial time, the second with the direction of the
    /// velocity at the initial time and the third with the normal of the
    /// circle's plane.
    pub orientation: Orientation,
    /// The position of the center of the circle.
    pub center_position: Position,
    /// The radius of the circle.
    pub radius: fph,
    /// The duration of one revolution.
    pub period: fph,
}

impl CircularTrajectoryComp {
    /// Creates a new component for a circular trajectory with the given
    /// properties.
    pub fn new(
        initial_time: fph,
        orientation: Orientation,
        center_position: Position,
        radius: fph,
        period: fph,
    ) -> Self {
        Self {
            initial_time,
            orientation,
            center_position,
            radius,
            period,
        }
    }
}

/// Registers all analytical motion
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_circular_motion_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, CircularTrajectoryComp)
}
