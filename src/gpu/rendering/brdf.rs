//! Bidirectional reflection distribution functions.

#![allow(non_snake_case)]

use crate::{
    gpu::{
        rendering::fre,
        texture::{DepthOrArrayLayers, TextureLookupTable},
    },
    num::Float,
};
use std::num::NonZeroU32;

/// Creates two tables of the specular GGX microfacet BRDF reflectance values
/// (the integral of the cosine-weighted BRDF over the positive hemisphere of
/// light directions) for the ranges of possible values for `v_dot_n` and
/// `roughness` In both tables, `v_dot_n` varies along the width and `roughness`
/// along the height. Their values can be used directly as UV texture
/// coordinates. The two tables are stored contiguously in memory.
///
/// The first table is created with the normal incidence Fresnel reflectance
/// `F0` equal to `0`, the second with `F0` equal to `1`. The BRDF reflectance
/// varies linearly between the reflectance for `F0 = 0` and `F0 = 1` so
/// performing linear interpolation of the reflectances sampled from the same
/// location in the two tables with `F0` as the interpolation weight gives the
/// exact result.
pub fn create_specular_ggx_reflectance_lookup_tables(
    num_v_dot_n_samples: usize,
    num_roughness_samples: usize,
) -> TextureLookupTable<f32> {
    const MIN_ROUGHNESS: fre = 0.05;

    assert!(num_v_dot_n_samples > 1);
    assert!(num_roughness_samples > 1);

    let v_dot_n_delta = 1.0 / ((num_v_dot_n_samples - 1) as fre);
    let roughness_delta = (1.0 - MIN_ROUGHNESS) / ((num_roughness_samples - 1) as fre);

    let mut data = Vec::with_capacity(2 * num_roughness_samples * num_v_dot_n_samples);

    for F0 in [0.0, 1.0] {
        let mut roughness = MIN_ROUGHNESS;

        for _ in 0..num_roughness_samples {
            let mut v_dot_n = 0.0;

            for _ in 0..num_v_dot_n_samples {
                let reflectance = compute_reflectance_integral(
                    |v_dot_n, l_dot_n, l_dot_v| {
                        evaluate_specular_ggx_brdf(F0, roughness, v_dot_n, l_dot_n, l_dot_v)
                    },
                    v_dot_n,
                );
                data.push(reflectance.clamp(0.0, 1.0));

                v_dot_n += v_dot_n_delta;
            }

            roughness += roughness_delta;
        }
    }

    TextureLookupTable::new(
        num_v_dot_n_samples,
        num_roughness_samples,
        DepthOrArrayLayers::ArrayLayers(NonZeroU32::new(2).unwrap()),
        data,
    )
}

/// Integrates the radiance reflected toward the view direction `v` from all
/// light directions `l` in the full hemisphere around the normal vector `n`.
/// The given radiance function should take the dot products `v_dot_n`,
/// `l_dot_n` and `l_dot_v` as its only arguments. The radiance function is
/// assumed isotropic.
pub fn compute_reflectance_integral(
    evaluate_radiance: impl Fn(fre, fre, fre) -> fre,
    v_dot_n: fre,
) -> fre {
    let evaluate_integrand_for_l_dot_n = |l_dot_n: fre| -> fre {
        let l_dot_v_offset = v_dot_n * l_dot_n;
        let l_dot_v_scale = fre::sqrt((1.0 - v_dot_n.powi(2)) * (1.0 - l_dot_n.powi(2)));

        let evaluate_integrand_for_phi = |phi: fre| -> fre {
            let l_dot_v = l_dot_v_offset + fre::cos(phi) * l_dot_v_scale;
            evaluate_radiance(v_dot_n, l_dot_n, l_dot_v)
        };

        l_dot_n * integrate_hundred_point_gauss_legendre(evaluate_integrand_for_phi, 0.0, fre::PI)
    };

    // Multiply with 2 since we only integrated over half the hemisphere (0 <=
    // phi <= pi)
    2.0 * integrate_hundred_point_gauss_legendre(evaluate_integrand_for_l_dot_n, 0.0, 1.0)
}

/// Evaluates the specular BRDF based on the GGX distribution of microfacet
/// normals.
pub fn evaluate_specular_ggx_brdf(
    F0: fre,
    roughness: fre,
    v_dot_n: fre,
    l_dot_n: fre,
    l_dot_v: fre,
) -> fre {
    let (n_dot_h, _, l_dot_h) = compute_half_vector_dot_products(v_dot_n, l_dot_n, l_dot_v);

    let fre = compute_fresnel_reflectance(F0, l_dot_h);
    let G = evaluate_scaled_ggx_masking_shadowing_function(v_dot_n, l_dot_n, roughness);
    let D = evaluate_ggx_distribution(n_dot_h, roughness);

    fre * G * D
}

/// Uses the definition of the half vector, `h = (l + v) / |l + v|`, to
/// efficiently compute the dot products of the light direction `l`, view
/// direction `v` and normal vector `n` with `h`.
fn compute_half_vector_dot_products(v_dot_n: fre, l_dot_n: fre, l_dot_v: fre) -> (fre, fre, fre) {
    let one_plus_l_dot_v = 1.0 + l_dot_v;
    let l_plus_v_squared_len = 2.0 * one_plus_l_dot_v;
    let inverse_l_plus_v_len = 1.0 / fre::sqrt(l_plus_v_squared_len);

    let l_dot_h = one_plus_l_dot_v * inverse_l_plus_v_len;
    let n_dot_h = (l_dot_n + v_dot_n) * inverse_l_plus_v_len;
    let v_dot_h = l_dot_h;

    (n_dot_h, v_dot_h, l_dot_h)
}

/// Computes the Fresnel reflectance for the given normal incidence Fresnel
/// reflectance `F0` and light direction cosine `l_dot_n`, using the Schlick
/// approximation.
///
/// # Note
/// For a specular microfacet BRDF, the half vector `h` should be used as the
/// normal vector in `l_dot_n` rather than the macroscopic normal vector `n`
fn compute_fresnel_reflectance(F0: fre, l_dot_n: fre) -> fre {
    F0 + (1.0 - F0) * (1.0 - fre::max(0.0, l_dot_n)).powi(5)
}

/// Evaluates the Smith height-correlated masking shadowing function `G` for the
/// GGX distribution of microfacet normals, divided by the `4 * |l_dot_n| *
/// |v_dot_n|` factor that occurs in the integral for computing the BRDF.
///
/// Uses the approximation of Hammon (2017).
fn evaluate_scaled_ggx_masking_shadowing_function(
    v_dot_n: fre,
    l_dot_n: fre,
    roughness: fre,
) -> fre {
    let abs_v_dot_n = v_dot_n.abs();
    let abs_l_dot_n = l_dot_n.abs();
    0.5 / ((1.0 - roughness) * (2.0 * abs_v_dot_n * abs_l_dot_n)
        + roughness * (abs_v_dot_n + abs_l_dot_n)
        + 1e-6)
}

/// Evaluates the GGX distribution of microfacet normals. The `m` vector in
/// `n_dot_m` is the microfacet normal, which for a specular BRDF should be the
/// half vector `h`.
fn evaluate_ggx_distribution(n_dot_m: fre, roughness: fre) -> fre {
    if n_dot_m > 0.0 {
        let roughness_squared = roughness.powi(2);
        roughness_squared
            / (fre::PI * (1.0 + n_dot_m.powi(2) * (roughness_squared - 1.0)).powi(2) + 1e-6)
    } else {
        0.0
    }
}

/// Estimates the integral of the given function over the given interval using a
/// hundred-point Gauss-Legendre quadrature.
fn integrate_hundred_point_gauss_legendre(
    evaluate_integrand: impl Fn(fre) -> fre,
    start: fre,
    end: fre,
) -> fre {
    #[rustfmt::skip]
    #[allow(clippy::excessive_precision)]
    const COORDS: &[fre] = &[
        -0.9997137268, -0.9984919506, -0.996295135, -0.993124937, -0.988984395,
        -0.983877541, -0.977809358, -0.970785776, -0.962813654, -0.953900783,
        -0.9440558701, -0.933288535, -0.921609298, -0.909029571, -0.895561645,
        -0.8812186794, -0.8660146885, -0.8499645279, -0.83308388, -0.8153892383,
        -0.796897892, -0.7776279097, -0.757598119, -0.7368280898, -0.7153381176,
        -0.6931491994, -0.6702830156, -0.6467619085, -0.6226088602, -0.5978474703,
        -0.572501933, -0.5465970121, -0.5201580199, -0.4932107892, -0.46578165,
        -0.4378974022, -0.4095852917, -0.3808729816, -0.351788526, -0.3223603439,
        -0.292617188, -0.2625881204, -0.2323024818, -0.2017898641, -0.171080081,
        -0.140203137, -0.109189204, -0.0780685828, -0.046871682, -0.015628984,
        0.0156289844, 0.0468716824, 0.0780685828, 0.1091892036, 0.1402031372,
        0.1710800805, 0.2017898641, 0.2323024818, 0.2625881204, 0.292617188,
        0.322360344, 0.351788526, 0.380872982, 0.4095852917, 0.437897402,
        0.46578165, 0.493210789, 0.5201580199, 0.5465970121, 0.5725019326,
        0.5978474703, 0.62260886, 0.646761909, 0.670283016, 0.693149199,
        0.7153381176, 0.7368280898, 0.7575981185, 0.7776279097, 0.7968978924,
        0.8153892383, 0.8330838799, 0.8499645279, 0.8660146885, 0.8812186794,
        0.895561645, 0.909029571, 0.921609298, 0.933288535, 0.94405587,
        0.9539007829, 0.962813654, 0.9707857758, 0.977809358, 0.9838775407,
        0.9889843952, 0.993124937, 0.996295135, 0.9984919506, 0.999713727,
    ];

    #[rustfmt::skip]
    #[allow(clippy::excessive_precision)]
    const WEIGHTS: &[fre] = &[
        7.346345E-4, 0.0017093927, 0.002683925372, 0.0036559612, 0.0046244501,
        0.005588428, 0.0065469485, 0.00749907326, 0.008443871, 0.0093804197,
        0.0103078026, 0.011225114, 0.0121314577, 0.013025948, 0.0139077107,
        0.014775885, 0.0156296211, 0.0164680862, 0.0172904606, 0.018095941,
        0.01888374, 0.0196530875, 0.02040323265, 0.0211334421, 0.0218430024,
        0.0225312203, 0.023197423, 0.02384096, 0.024461203, 0.025057544,
        0.025629403, 0.0261762192, 0.0266974592, 0.02719261345, 0.0276611982,
        0.028102756, 0.0285168543, 0.02890309, 0.02926108411, 0.0295904881,
        0.02989098, 0.0301622651, 0.0304040795, 0.030616187, 0.030798379,
        0.0309504789, 0.0310723374, 0.031163836, 0.0312248843, 0.0312554235,
        0.031255423, 0.03122488425, 0.031163836, 0.0310723374, 0.0309504789,
        0.030798379, 0.0306161866, 0.0304040795, 0.030162265, 0.0298909796,
        0.0295904881, 0.029261084, 0.02890309, 0.028516854, 0.0281027557,
        0.0276611982, 0.027192613, 0.026697459, 0.0261762192, 0.0256294029,
        0.0250575445, 0.0244612027, 0.0238409603, 0.023197423, 0.0225312203,
        0.0218430024, 0.0211334421, 0.020403233, 0.0196530875, 0.01888374,
        0.0180959407, 0.017290461, 0.0164680862, 0.0156296211, 0.0147758845,
        0.0139077107, 0.0130259479, 0.012131458, 0.011225114, 0.01030780258,
        0.00938042, 0.0084438715, 0.0074990733, 0.0065469485, 0.005588428,
        0.00462445006, 0.0036559612, 0.0026839254, 0.00170939265, 7.346345E-4,
    ];

    assert!(
        end >= start,
        "Interval end {} is smaller than interval start {}",
        end,
        start
    );
    let interval_scale = 0.5 * (end - start);
    let interval_offset = 0.5 * (end + start);

    let mut integral = 0.0;

    for idx in 0..100 {
        integral +=
            WEIGHTS[idx] * evaluate_integrand(interval_offset + interval_scale * COORDS[idx]);
    }

    integral * interval_scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn gauss_legendre_integration_works() {
        assert_abs_diff_eq!(
            integrate_hundred_point_gauss_legendre(fre::sin, 0.0, fre::TWO_PI),
            0.0
        );
        assert_abs_diff_eq!(
            integrate_hundred_point_gauss_legendre(fre::sin, 0.0, fre::PI),
            2.0
        );
        assert_abs_diff_eq!(
            integrate_hundred_point_gauss_legendre(fre::exp, -1.0, 1.0),
            fre::exp(1.0) - 1.0 / fre::exp(1.0),
            epsilon = 1e-6
        );
        assert_abs_diff_eq!(
            integrate_hundred_point_gauss_legendre(
                |x| fre::exp(x) * fre::sin(x).powi(2),
                -1.0,
                0.5
            ),
            0.191658,
            epsilon = 1e-6
        );
    }
}
