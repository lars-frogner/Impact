//! Physical constants.

pub mod f64 {
    /// m/s
    pub const LIGHT_SPEED: f64 = 299792458.0;

    /// m² kg/s²/K
    pub const BOLTZMANN_CONSTANT: f64 = 1.380649e-23;

    /// m² kg/s
    pub const PLANCK_CONSTANT: f64 = 6.62607015e-34;

    /// W/m²/K⁴
    pub const STEFAN_BOLTZMANN_CONSTANT: f64 = 5.670374419e-8;
}

pub mod f32 {
    /// m/s
    pub const LIGHT_SPEED: f32 = super::f64::LIGHT_SPEED as f32;

    /// m² kg/s²/K
    pub const BOLTZMANN_CONSTANT: f32 = super::f64::BOLTZMANN_CONSTANT as f32;

    /// m² kg/s
    pub const PLANCK_CONSTANT: f32 = super::f64::PLANCK_CONSTANT as f32;

    /// W/m²/K⁴
    pub const STEFAN_BOLTZMANN_CONSTANT: f32 = super::f64::STEFAN_BOLTZMANN_CONSTANT as f32;
}
