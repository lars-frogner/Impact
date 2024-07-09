//! [`Component`](impact_ecs::component::Component)s related to uniform
//! gravitational acceleration.

use crate::{component::ComponentRegistry, physics::fph};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{vector, Vector3};

/// [`Component`](impact_ecs::component::Component) for entities that have a
/// uniform gravitational acceleration.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformGravityComp {
    /// The gravitational acceleration of the entity.
    pub acceleration: Vector3<fph>,
}

impl UniformGravityComp {
    /// The downward acceleration at the surface of Earth [m/s^2].
    pub const EARTH_DOWNWARD_ACCELERATION: fph = 9.81;

    /// Creates a new component for uniform gravitational acceleration.
    pub fn new(acceleration: Vector3<fph>) -> Self {
        Self { acceleration }
    }

    /// Creates a new component for uniform gravitational acceleration in the
    /// negative y-direction.
    pub fn downward(acceleration: fph) -> Self {
        Self::new(vector![0.0, -acceleration, 0.0])
    }

    /// Creates a new component for the gravitational acceleration at the
    /// surface of Earth.
    pub fn earth() -> Self {
        Self::downward(Self::EARTH_DOWNWARD_ACCELERATION)
    }
}

/// Registers all uniform gravity force
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_uniform_gravity_force_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, UniformGravityComp)
}
