//! Physical media objects can interact with.

use crate::physics::{fph, motion::Velocity};

/// A physical medium with the same properties and state everywhere.
#[derive(Clone, Debug)]
pub struct UniformMedium {
    /// The mass density of the medium.
    pub mass_density: fph,
    /// The velocity of the medium.
    pub velocity: Velocity,
}

impl UniformMedium {
    /// Earth air mass density at sea level and room temperature [kg/m^3].
    pub const SEA_LEVEL_AIR_MASS_DENSITY: fph = 1.2;

    /// Water mass density [kg/m^3].
    pub const WATER_MASS_DENSITY: fph = 1e3;

    /// Creates a new uniform medium with the given mass density and velocity.
    pub fn new(mass_density: fph, velocity: Velocity) -> Self {
        Self {
            mass_density,
            velocity,
        }
    }

    /// Creates a new vacuum medium (zero mass density and velocity).
    pub fn vacuum() -> Self {
        Self::new(0.0, Velocity::zeros())
    }

    /// Creates a new medium of Earth air at sea level and room temperature with
    /// no wind.
    pub fn still_air() -> Self {
        Self::moving_air(Velocity::zeros())
    }

    /// Creates a new medium of Earth air at sea level and room temperature with
    /// the given wind velocity.
    pub fn moving_air(velocity: Velocity) -> Self {
        Self::new(Self::SEA_LEVEL_AIR_MASS_DENSITY, velocity)
    }

    /// Creates a new medium of water with no flow.
    pub fn still_water() -> Self {
        Self::moving_water(Velocity::zeros())
    }

    /// Creates a new medium of water with the given flow velocity.
    pub fn moving_water(velocity: Velocity) -> Self {
        Self::new(Self::WATER_MASS_DENSITY, velocity)
    }
}
