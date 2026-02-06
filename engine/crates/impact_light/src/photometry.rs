//! Computation of photometric quantities.

use crate::Luminance;
use anyhow::{Result, anyhow};
use impact_alloc::{self, AVec, Allocator, Global};
use impact_math::consts::physics::f64::{BOLTZMANN_CONSTANT, LIGHT_SPEED, PLANCK_CONSTANT};
use std::{fs, path::Path, str::FromStr, sync::LazyLock};

static COLOR_MATCHING_FUNCTIONS: LazyLock<ColorMatchingFunctions<Global>> = LazyLock::new(|| {
    ColorMatchingFunctions::load_from_csv_str(Global, include_str!("../data/CIE_xyz_1931_2deg.csv"))
        .expect("Failed to load color matching functions")
});

/// A mapping from wavelength to spectral radiance (W/sr/m²).
pub trait RadianceSpectrum {
    /// Evaluates the spectral radiance for the given wavelength in nm.
    fn eval(&self, wavelength_in_nm: f32) -> f32;
}

/// Planck's function for the spectral radiance of a black body.
#[derive(Clone, Copy, Debug)]
pub struct BlackBodySpectrum {
    pub temperature: f32,
    pub exponent_scale: f32,
}

/// CIE 1931 color matching functions.
#[derive(Clone, Debug)]
pub struct ColorMatchingFunctions<A: Allocator = Global> {
    points: AVec<ColorMatchingPoint, A>,
}

#[derive(Clone, Copy, Debug)]
struct ColorMatchingPoint {
    /// In nanometers.
    wavelength: f32,
    x: f32,
    y: f32,
    z: f32,
}

impl<F> RadianceSpectrum for F
where
    F: Fn(f32) -> f32,
{
    fn eval(&self, wavelength_in_nm: f32) -> f32 {
        self(wavelength_in_nm)
    }
}

impl BlackBodySpectrum {
    const SCALE: f32 = (2.0 * PLANCK_CONSTANT * LIGHT_SPEED * LIGHT_SPEED * 1e45) as f32;

    /// Constructs the spectrum of a black body with the given temperature in
    /// kelvin.
    pub fn new(temperature: f32) -> Self {
        let exponent_scale = (PLANCK_CONSTANT * LIGHT_SPEED * 1e9
            / (BOLTZMANN_CONSTANT * f64::from(temperature))) as f32;

        Self {
            temperature,
            exponent_scale,
        }
    }
}

impl RadianceSpectrum for BlackBodySpectrum {
    fn eval(&self, wavelength_in_nm: f32) -> f32 {
        let inv_wavelength = wavelength_in_nm.recip();

        Self::SCALE * inv_wavelength.powi(5)
            / (f32::exp(self.exponent_scale * inv_wavelength) - 1.0)
    }
}

impl<A: Allocator> ColorMatchingFunctions<A> {
    /// Loads the CIE 1931 colour matching function values from the CSV (from
    /// <https://cie.co.at/datatable/cie-1931-colour-matching-functions-2-degree-observer>)
    /// at the given path.
    ///
    /// # Errors
    /// Returns an error if opening or parsing the CSV fails.
    pub fn load_from_csv_file(alloc: A, path: &Path) -> Result<Self> {
        let csv = fs::read_to_string(path)?;
        Self::load_from_csv_str(alloc, &csv)
    }

    /// Loads the CIE 1931 colour matching function values from the given
    /// CSV-formatted string (from
    /// <https://cie.co.at/datatable/cie-1931-colour-matching-functions-2-degree-observer>).
    ///
    /// # Errors
    /// Returns an error if parsing the CSV fails.
    pub fn load_from_csv_str(alloc: A, csv: &str) -> Result<Self> {
        let mut points = AVec::new_in(alloc);

        for line in csv.lines() {
            let mut splitted = line.split(',');

            let wavelength = splitted
                .next()
                .ok_or_else(|| anyhow!("Missing wavelength"))?;

            let x = splitted.next().ok_or_else(|| anyhow!("Missing x"))?;
            let y = splitted.next().ok_or_else(|| anyhow!("Missing y"))?;
            let z = splitted.next().ok_or_else(|| anyhow!("Missing z"))?;

            points.push(ColorMatchingPoint {
                wavelength: f32::from_str(wavelength)?,
                x: f32::from_str(x)?,
                y: f32::from_str(y)?,
                z: f32::from_str(z)?,
            });
        }

        Ok(Self { points })
    }

    /// Integrates the given spectral radiance function weighted by the three
    /// color matching functions over the visible wavelength range (360 to 830
    /// nm) to obtain the tristimulus X, Y and Z values for the spectrum.
    ///
    /// Use [`luminance_from_tristimulus_y`] to convert the returned Y value to
    /// luminance.
    pub fn compute_cie_xyz_for_spectrum(
        &self,
        radiance_spectrum: &impl RadianceSpectrum,
    ) -> (f32, f32, f32) {
        assert!(self.points.len() >= 2);

        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        // Integrate using the trapezoidal rule

        let mut accumulate = |point: &ColorMatchingPoint, wavelength_diff: f32| {
            let radiance = radiance_spectrum.eval(point.wavelength);
            x += point.x * radiance * wavelength_diff;
            y += point.y * radiance * wavelength_diff;
            z += point.z * radiance * wavelength_diff;
        };

        let first_point = &self.points[0];
        let next_point = &self.points[1];
        accumulate(first_point, next_point.wavelength - first_point.wavelength);

        for points in self.points.windows(3) {
            accumulate(&points[1], points[2].wavelength - points[0].wavelength);
        }

        let last_point = &self.points[self.points.len() - 1];
        let prev_point = &self.points[self.points.len() - 2];
        accumulate(last_point, last_point.wavelength - prev_point.wavelength);

        // Divide by two and scale from nm wavelengths to m
        x *= 0.5 * 1e-9;
        y *= 0.5 * 1e-9;
        z *= 0.5 * 1e-9;

        (x, y, z)
    }

    /// Integrates the given spectral radiance function weighted by the three
    /// color matching functions over the visible wavelength range (360 to 830
    /// nm) to obtain the tristimulus X, Y and Z values for the spectrum, then
    /// converts this to a luminance RGB triplet.
    pub fn compute_rgb_luminance_for_spectrum(
        &self,
        radiance_spectrum: &impl RadianceSpectrum,
    ) -> Luminance {
        let (x, y, z) = self.compute_cie_xyz_for_spectrum(radiance_spectrum);
        rgb_luminance_from_tristimulus(x, y, z)
    }
}

/// Converts a tristimulus Y value to total luminance.
pub const fn luminance_from_tristimulus_y(y: f32) -> f32 {
    683.0 * y
}

/// Converts the given tristimulus XYZ triple to RGB luminance.
pub const fn rgb_luminance_from_tristimulus(x: f32, y: f32, z: f32) -> Luminance {
    // Convert to luminance
    let x = 683.0 * x;
    let y = 683.0 * y;
    let z = 683.0 * z;

    // Transform to linear sRGB
    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b = 0.0557 * x - 0.2040 * y + 1.0570 * z;

    Luminance::new(r, g, b)
}

/// Computes the total luminance for the given luminance RGB triplet.
pub const fn total_luminance_from_rgb(luminance: &Luminance) -> f32 {
    // Compute Y from RGB
    0.212586 * luminance.x() + 0.71517 * luminance.y() + 0.0722005 * luminance.z()
}

/// Computes the RGB luminance of a black body with the given temperature in
/// kelvin.
pub fn compute_black_body_luminance(temperature: f32) -> Luminance {
    COLOR_MATCHING_FUNCTIONS
        .compute_rgb_luminance_for_spectrum(&BlackBodySpectrum::new(temperature))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computed_solar_luminance_is_reasonable() {
        let luminance = compute_black_body_luminance(5800.0);
        assert!((1.5e9..=2.0e9).contains(&total_luminance_from_rgb(&luminance)));
    }
}
