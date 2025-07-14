//! Physical media objects can interact with.

use crate::{fph, quantities::Velocity};
use roc_integration::roc;

/// A physical medium with the same properties and state everywhere.
#[roc(parents = "Physics")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct UniformMedium {
    /// The mass density of the medium.
    pub mass_density: fph,
    /// The velocity of the medium.
    pub velocity: Velocity,
}

#[roc]
impl UniformMedium {
    /// Earth air mass density at sea level and room temperature [kg/m^3].
    #[roc(expr = "1.2")]
    pub const SEA_LEVEL_AIR_MASS_DENSITY: fph = 1.2;

    /// Water mass density [kg/m^3].
    #[roc(expr = "1e3")]
    pub const WATER_MASS_DENSITY: fph = 1e3;

    /// Creates a new uniform medium with the given mass density and velocity.
    #[roc(body = "{ mass_density, velocity }")]
    pub fn new(mass_density: fph, velocity: Velocity) -> Self {
        Self {
            mass_density,
            velocity,
        }
    }

    /// Creates a new vacuum medium (zero mass density and velocity).
    #[roc(body = "new(0.0, Vector3.zero)")]
    pub fn vacuum() -> Self {
        Self::new(0.0, Velocity::zeros())
    }

    /// Creates a new medium of Earth air at sea level and room temperature with
    /// no wind.
    #[roc(body = "moving_air(Vector3.zero)")]
    pub fn still_air() -> Self {
        Self::moving_air(Velocity::zeros())
    }

    /// Creates a new medium of Earth air at sea level and room temperature with
    /// the given wind velocity.
    #[roc(body = "new(sea_level_air_mass_density, velocity)")]
    pub fn moving_air(velocity: Velocity) -> Self {
        Self::new(Self::SEA_LEVEL_AIR_MASS_DENSITY, velocity)
    }

    /// Creates a new medium of water with no flow.
    #[roc(body = "moving_water(Vector3.zero)")]
    pub fn still_water() -> Self {
        Self::moving_water(Velocity::zeros())
    }

    /// Creates a new medium of water with the given flow velocity.
    #[roc(body = "new(water_mass_density, velocity)")]
    pub fn moving_water(velocity: Velocity) -> Self {
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

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for UniformMedium {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            mass_density: fph::arbitrary(u)?,
            velocity: Velocity::new(fph::arbitrary(u)?, fph::arbitrary(u)?, fph::arbitrary(u)?),
        })
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        let size = 4 * std::mem::size_of::<fph>();
        (size, Some(size))
    }
}
