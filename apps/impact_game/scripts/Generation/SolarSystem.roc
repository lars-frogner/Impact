module [
    Spec,
    BodyDistributions,
    System,
    Properties,
    Star,
    Body,
    generate,
]

import core.Vector3 exposing [Vector3]
import core.Point3 exposing [Point3]
import core.UnitVector3
import core.UnitQuaternion
import core.Random
import core.Radians

import Generation.Orbit as Orbit

Spec : {
    number_of_bodies : U64,
    body_distributions : BodyDistributions,
    star_radius : F32,
    star_mass_density : F32,
    max_orbital_period : F32,
    min_body_illuminance : F32,
}

BodyDistributions : {
    size : Random.PowerLaw,
    semi_major_axis : Random.PowerLaw,
    eccentricity : Random.Gaussian,
    inclination_angle : Random.Gaussian,
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
    system_radius = spec.body_distributions.semi_major_axis.max_value

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
            |(rng, body_list), _idx|
                (next_rng, body) = generate_body(
                    rng,
                    spec.body_distributions,
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

generate_body : Random.Rng, BodyDistributions, F32, F32 -> (Random.Rng, Body)
generate_body = |rng, distributions, grav_const, star_mass|
    (rng2, size) = Random.gen_f32_power_law(rng, distributions.size)

    (rng3, semi_major_axis) = Random.gen_f32_power_law(rng2, distributions.semi_major_axis)

    (rng4, eccentricity_signed) = Random.gen_f32_gaussian(rng3, distributions.eccentricity)
    eccentricity = Num.min(1.0, Num.abs(eccentricity_signed))

    (rng5, azimuthal_angle) = Random.gen_f32_in_range(rng4, 0, 2 * Num.pi)

    (rng6, inclination_angle_deg) = Random.gen_f32_gaussian(rng5, distributions.inclination_angle)
    inclination_angle = Radians.from_degrees(inclination_angle_deg)

    orientation =
        UnitQuaternion.from_axis_angle(UnitVector3.unit_x, (-Num.pi) / 2)
        |> UnitQuaternion.mul(UnitQuaternion.from_axis_angle(UnitVector3.unit_z, azimuthal_angle))
        |> UnitQuaternion.mul(UnitQuaternion.from_axis_angle(UnitVector3.unit_x, inclination_angle))

    period = Orbit.compute_orbital_period(grav_const, star_mass, semi_major_axis)

    (rng7, time) = Random.gen_f32_in_range(rng6, 0, period)

    orbit = {
        periapsis_time: 0.0,
        orientation,
        focal_position: Point3.origin,
        semi_major_axis,
        eccentricity,
        period,
    }

    (position, velocity) = Orbit.compute_position_and_velocity(orbit, time)

    (rng7, { position, velocity, size })

compute_sphere_mass = |radius, mass_density|
    (4.0 / 3.0) * Num.pi * Num.pow(radius, 3) * mass_density

compute_grav_const = |star_mass, distance, orbital_period|
    Num.pow(2 * Num.pi, 2) * Num.pow(distance, 3) / (star_mass * Num.pow(orbital_period, 2))

compute_sphere_emissive_luminance = |luminous_intensity, radius|
    luminous_intensity / (Num.pi * Num.pow(radius, 2))

compute_luminous_intensity = |illuminance, distance|
    illuminance * Num.pow(distance, 2)
