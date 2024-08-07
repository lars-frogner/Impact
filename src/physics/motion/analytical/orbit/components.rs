//! [`Component`](impact_ecs::component::Component)s related to orbital motion.

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

/// [`Component`](impact_ecs::component::Component) for entities that follow an
/// closed orbital trajectory over time.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp) and a
/// [`VelocityComp`](crate::physics::VelocityComp).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OrbitalTrajectoryComp {
    /// When (in simulation time) the entity should be at the periapsis (the
    /// closest point to the orbited body).
    pub periapsis_time: fph,
    /// The orientation of the orbit. The first axis of the oriented orbit frame
    /// will coincide with the direction from the orbited body to the periapsis,
    /// the second with the direction of the velocity at the periapsis and the
    /// third with the normal of the orbital plane.
    pub orientation: Orientation,
    /// The position of the focal point where the body being orbited would be
    /// located.
    pub focal_position: Position,
    /// Half the longest diameter of the orbital ellipse.
    pub semi_major_axis: fph,
    /// The eccentricity of the orbital ellipse (0 is circular, 1 is a line).
    pub eccentricity: fph,
    /// The orbital period.
    pub period: fph,
}

impl OrbitalTrajectoryComp {
    /// Creates a new component for an orbital trajectory with the given
    /// properties.
    pub fn new(
        periapsis_time: fph,
        orientation: Orientation,
        focal_position: Position,
        semi_major_axis: fph,
        eccentricity: fph,
        period: fph,
    ) -> Self {
        Self {
            periapsis_time,
            orientation,
            focal_position,
            semi_major_axis,
            eccentricity,
            period,
        }
    }
}

/// Registers all orbital motion
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_orbital_motion_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, OrbitalTrajectoryComp)
}
