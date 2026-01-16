//! Physical media objects can interact with.

use crate::quantities::VelocityC;
use roc_integration::roc;

/// A physical medium with the same properties and state everywhere.
#[roc(parents = "Physics")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub struct UniformMedium {
    /// The mass density of the medium.
    pub mass_density: f32,
    /// The velocity of the medium.
    pub velocity: VelocityC,
}

#[roc]
impl UniformMedium {
    /// Earth air mass density at sea level and room temperature [kg/m^3].
    #[roc(expr = "1.2")]
    pub const SEA_LEVEL_AIR_MASS_DENSITY: f32 = 1.2;

    /// Water mass density [kg/m^3].
    #[roc(expr = "1e3")]
    pub const WATER_MASS_DENSITY: f32 = 1e3;

    /// Creates a new uniform medium with the given mass density and velocity.
    #[roc(body = "{ mass_density, velocity }")]
    pub fn new(mass_density: f32, velocity: VelocityC) -> Self {
        Self {
            mass_density,
            velocity,
        }
    }

    /// Creates a new vacuum medium (zero mass density and velocity).
    #[roc(body = "new(0.0, Vector3.zeros)")]
    pub fn vacuum() -> Self {
        Self::new(0.0, VelocityC::zeros())
    }

    /// Creates a new medium of Earth air at sea level and room temperature with
    /// no wind.
    #[roc(body = "moving_air(Vector3.zeros)")]
    pub fn still_air() -> Self {
        Self::moving_air(VelocityC::zeros())
    }

    /// Creates a new medium of Earth air at sea level and room temperature with
    /// the given wind velocity.
    #[roc(body = "new(sea_level_air_mass_density, velocity)")]
    pub fn moving_air(velocity: VelocityC) -> Self {
        Self::new(Self::SEA_LEVEL_AIR_MASS_DENSITY, velocity)
    }

    /// Creates a new medium of water with no flow.
    #[roc(body = "moving_water(Vector3.zeros)")]
    pub fn still_water() -> Self {
        Self::moving_water(VelocityC::zeros())
    }

    /// Creates a new medium of water with the given flow velocity.
    #[roc(body = "new(water_mass_density, velocity)")]
    pub fn moving_water(velocity: VelocityC) -> Self {
        Self::new(Self::WATER_MASS_DENSITY, velocity)
    }
}

impl Default for UniformMedium {
    fn default() -> Self {
        Self::vacuum()
    }
}

impl PartialEq for UniformMedium {
    fn eq(&self, other: &Self) -> bool {
        self.mass_density.to_bits() == other.mass_density.to_bits()
            && bytemuck::bytes_of(&self.velocity) == bytemuck::bytes_of(&other.velocity)
    }
}

impl Eq for UniformMedium {}
