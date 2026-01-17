module [
    Spec,
    System,
    Properties,
    Star,
    Body,
    PowerLaw,
    generate,
]

import core.Vector3 exposing [Vector3]
import core.Point3 exposing [Point3]
import core.Random

Spec : {
    number_of_bodies : U64,
    body_size_distr : PowerLaw,
    body_distance_distr : PowerLaw,
    star_radius : F32,
    star_mass_density : F32,
    max_orbital_period : F32,
    min_body_illuminance : F32,
}

System : {
    properties : Properties,
    star : Star,
    bodies : List Body,
}

Properties : {
    grav_const : F32,
    radius : F32,
}

Star : {
    radius : F32,
    mass_density : F32,
    luminous_intensity : F32,
    emissive_luminance : F32,
}

Body : {
    position : Point3,
    velocity : Vector3,
    size : F32,
}

generate : Spec, U64 -> System
generate = |spec, seed|
    system_radius = spec.body_distance_distr.max_value

    star_mass = compute_sphere_mass(spec.star_radius, spec.star_mass_density)

    grav_const = compute_grav_const(
        star_mass,
        system_radius,
        spec.max_orbital_period,
    )

    star_luminous_intensity = compute_luminous_intensity(
        spec.min_body_illuminance,
        system_radius,
    )
    star_emissive_luminance = compute_sphere_emissive_luminance(
        star_luminous_intensity,
        spec.star_radius,
    )

    init_rng = Random.new_rng(seed)

    (_, bodies) =
        List.range({ start: At 0, end: Length spec.number_of_bodies })
        |> List.walk(
            (init_rng, List.with_capacity(spec.number_of_bodies)),
            |(rng, body_list), idx|
                (next_rng, body) = generate_body(
                    rng,
                    spec.body_size_distr,
                    spec.body_distance_distr,
                    grav_const,
                    star_mass,
                )
                (next_rng, body_list |> List.append(body)),
        )

    properties = {
        grav_const,
        radius: system_radius,
    }

    star = {
        radius: spec.star_radius,
        mass_density: spec.star_mass_density,
        luminous_intensity: star_luminous_intensity,
        emissive_luminance: star_emissive_luminance,
    }

    { properties, star, bodies }

generate_body : Random.Rng, PowerLaw, PowerLaw, F32, F32 -> (Random.Rng, Body)
generate_body = |rng, size_distr, distance_distr, grav_const, star_mass|
    (rng2, size_prob) = Random.gen_f32(rng)
    size = eval_inverse_cumulative_power_law(size_distr, size_prob)

    (rng3, azimuthal_angle) = Random.gen_f32_in_range(rng2, 0, 2 * Num.pi)
    (rng4, distance_prob) = Random.gen_f32(rng3)
    distance = eval_inverse_cumulative_power_law(distance_distr, distance_prob)

    orbital_speed = compute_stable_orbital_speed(grav_const, star_mass, distance)

    cos_azimuthal_angle = Num.cos(azimuthal_angle)
    sin_azimuthal_angle = Num.sin(azimuthal_angle)

    position = (
        distance * cos_azimuthal_angle,
        0.0,
        distance * sin_azimuthal_angle,
    )

    velocity = (
        (-orbital_speed) * sin_azimuthal_angle,
        0.0,
        orbital_speed * cos_azimuthal_angle,
    )

    (rng4, { position, velocity, size })

compute_sphere_mass = |radius, mass_density|
    (4.0 / 3.0) * Num.pi * Num.pow(radius, 3) * mass_density

compute_stable_orbital_speed = |grav_const, star_mass, distance|
    Num.sqrt(grav_const * star_mass / distance)

compute_stable_orbital_period = |distance, orbital_speed|
    2 * Num.pi * distance / orbital_speed

compute_grav_const = |star_mass, distance, orbital_period|
    Num.pow(2 * Num.pi, 2) * Num.pow(distance, 3) / (star_mass * Num.pow(orbital_period, 2))

compute_sphere_emissive_luminance = |luminous_intensity, radius|
    luminous_intensity / (Num.pi * Num.pow(radius, 2))

compute_luminous_intensity = |illuminance, distance|
    illuminance * Num.pow(distance, 2)

PowerLaw : {
    exponent : F32,
    min_value : F32,
    max_value : F32,
}

eval_power_law_prob : PowerLaw, F32 -> F32
eval_power_law_prob = |{ exponent, min_value, max_value }, value|
    exponent_p1 = exponent + 1
    if Num.abs(exponent_p1) > 1e-3 then
        min_value_pow = Num.pow(min_value, exponent_p1)
        max_value_pow = Num.pow(max_value, exponent_p1)
        (exponent_p1 / (max_value_pow - min_value_pow)) * Num.pow(value, exponent)
    else
        value / Num.log(max_value / min_value) # exponent = -1

eval_inverse_cumulative_power_law : PowerLaw, _ -> _
eval_inverse_cumulative_power_law = |{ exponent, min_value, max_value }, cumul_prob|
    exponent_p1 = exponent + 1
    if Num.abs(exponent_p1) > 1e-3 then
        min_value_pow = Num.pow(min_value, exponent_p1)
        max_value_pow = Num.pow(max_value, exponent_p1)
        Num.pow((max_value_pow - min_value_pow) * cumul_prob + min_value_pow, 1 / exponent_p1)
    else
        min_value * Num.pow(max_value / min_value, cumul_prob) # exponent = -1
