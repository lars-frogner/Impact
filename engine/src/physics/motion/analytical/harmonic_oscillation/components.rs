//! [`Component`](impact_ecs::component::Component)s related to harmonically
//! oscillating trajectories.

use crate::physics::{
    fph,
    motion::{Direction, Position},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use roc_integration::roc;

/// [`Component`](impact_ecs::component::Component) for entities that follow a
/// trajectory with harmonically oscillating position, velocity and acceleration
/// over time.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
/// [`VelocityComp`](crate::physics::VelocityComp).
#[roc(parents = "Comp", name = "HarmonicOscillatorTrajectory")]
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

#[roc]
impl HarmonicOscillatorTrajectoryComp {
    /// Creates a new component for an harmonically oscillating trajectory with
    /// the given properties.
    #[roc(body = r#"
    {
        center_time,
        center_position,
        direction,
        amplitude,
        period,
    }
    "#)]
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
