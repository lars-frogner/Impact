//! Power-law distribution.

use approx::abs_diff_eq;

/// Converts a uniformly distributed random fraction to a value between `min`
/// and `max` following a power-law distribution with the given exponent.
pub fn sample_power_law(min: f32, max: f32, exponent: f32, random_fraction: f32) -> f32 {
    let a = 1.0 - exponent;

    if abs_diff_eq!(a.abs(), 0.0) {
        // Special case for unit exponent
        min * (max / min).powf(random_fraction)
    } else {
        let min_pow = min.powf(a);
        let max_pow = max.powf(a);
        (min_pow + random_fraction * (max_pow - min_pow)).powf(1.0 / a)
    }
}
