//! [`Component`](impact_ecs::component::Component)s related to harmonically
//! oscillating trajectories.

use crate::{
    component::ComponentRegistry,
    physics::{
        fph,
        motion::{Direction, Position},
    },
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that follow a
/// trajectory with harmonically oscillating position, velocity and acceleration
/// over time.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
/// [`VelocityComp`](crate::physics::VelocityComp).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct HarmonicOscillatorTrajectoryComp {
    /// A simulation time when the entity should be at the center of
    /// oscillation.
    pub center_time: fph,
    /// The position of the center of oscillation.
    pub center_position: Position,
    /// The direction in which the entity is oscillating back and forth.
    pub direction: Direction,
    /// The maximum distance of the entity from the center position.
    pub amplitude: fph,
    /// The duration of one full oscillation.
    pub period: fph,
}

impl HarmonicOscillatorTrajectoryComp {
    /// Creates a new component for an harmonically oscillating trajectory with
    /// the given properties.
    pub fn new(
        center_time: fph,
        center_position: Position,
        direction: Direction,
        amplitude: fph,
        period: fph,
    ) -> Self {
        Self {
            center_time,
            center_position,
            direction,
            amplitude,
            period,
        }
    }
}

/// Registers all harmonic oscillation motion
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_harmonic_oscillation_motion_components(
    registry: &mut ComponentRegistry,
) -> Result<()> {
    register_component!(registry, HarmonicOscillatorTrajectoryComp)
}
