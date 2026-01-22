module [
    Orbit,
    compute_position_and_velocity,
    compute_orbital_period,
    compute_mean_orbital_speed,
]

import core.Vector3 exposing [Vector3]
import core.Point3 exposing [Point3]
import core.UnitQuaternion exposing [UnitQuaternion]
import core.NumUtil

Orbit : {
    # When (in simulation time) the orbiting body should be at the periapsis
    # (the closest point to the orbited body).
    periapsis_time : F32,
    # The orientation of the orbit. The first axis of the oriented orbit frame
    # will coincide with the direction from the orbited body to the periapsis,
    # the second with the direction of the velocity at the periapsis and the
    # third with the normal of the orbital plane.
    orientation : UnitQuaternion,
    # The position of the focal point where the body being orbited would be
    # located.
    focal_position : Point3,
    # Half the longest diameter of the orbital ellipse.
    semi_major_axis : F32,
    # The eccentricity of the orbital ellipse (0 is circular, 1 is a line).
    eccentricity : F32,
    # The orbital period.
    period : F32,
}

## Computes the period of an orbit with the given semi-major axis around a body
## with the given mass.
compute_orbital_period : F32, F32, F32 -> F32
compute_orbital_period = |grav_const, mass, semi_major_axis|
    2 * Num.pi * Num.sqrt(Num.pow(semi_major_axis, 3) / (grav_const * mass))

## Computes the mean speed in an orbit with the given semi-major axis around a
## body with the given mass.
compute_mean_orbital_speed : F32, F32, F32 -> F32
compute_mean_orbital_speed = |grav_const, mass, semi_major_axis|
    Num.sqrt(grav_const * mass / semi_major_axis)

## Computes the position and velocity of the body in the orbit at the given
## time.
compute_position_and_velocity : Orbit, F32 -> (Point3, Vector3)
compute_position_and_velocity = |orbit, time|
    expect orbit.semi_major_axis > 0.0
    expect orbit.eccentricity >= 0.0
    expect orbit.eccentricity < 1.0
    expect orbit.period > 0.0

    mean_angular_speed = compute_mean_angular_speed(orbit.period)

    mean_anomaly = compute_mean_anomaly(orbit.periapsis_time, mean_angular_speed, time)

    eccentric_anomaly = compute_eccentric_anomaly(orbit.eccentricity, mean_anomaly)

    (cos_true_anomaly, true_anomaly_per_eccentric_anomaly) = compute_cos_true_anomaly_and_true_anomaly_per_eccentric_anomaly(
        orbit.eccentricity,
        eccentric_anomaly,
    )

    orbital_distance = compute_orbital_distance(
        orbit.semi_major_axis,
        orbit.eccentricity,
        cos_true_anomaly,
    )

    sin_true_anomaly = compute_sin_true_anomaly(
        eccentric_anomaly,
        cos_true_anomaly,
    )

    orbital_displacement = compute_orbital_displacement(
        cos_true_anomaly,
        sin_true_anomaly,
        orbital_distance,
    )

    world_space_orbital_displacement = UnitQuaternion.rotate_vector(orbit.orientation, orbital_displacement)

    world_space_orbital_position = Point3.translate(orbit.focal_position, world_space_orbital_displacement)

    rate_of_change_of_true_anomaly = compute_rate_of_change_of_true_anomaly(
        orbit.eccentricity,
        mean_angular_speed,
        eccentric_anomaly,
        true_anomaly_per_eccentric_anomaly,
    )

    radial_speed = compute_radial_speed(
        orbit.semi_major_axis,
        orbit.eccentricity,
        cos_true_anomaly,
        sin_true_anomaly,
        rate_of_change_of_true_anomaly,
    )

    tangential_speed = compute_tangential_speed(orbital_distance, rate_of_change_of_true_anomaly)

    orbital_velocity = compute_orbital_velocity(
        cos_true_anomaly,
        sin_true_anomaly,
        radial_speed,
        tangential_speed,
    )

    world_space_orbital_velocity = UnitQuaternion.rotate_vector(orbit.orientation, orbital_velocity)

    (
        world_space_orbital_position,
        world_space_orbital_velocity,
    )

compute_mean_angular_speed = |period|
    Num.tau / period

compute_mean_anomaly = |periapsis_time, mean_angular_speed, time|
    NumUtil.modulo(mean_angular_speed * (time - periapsis_time), Num.tau)

compute_eccentric_anomaly = |eccentricity, mean_anomaly|
    eccentric_anomaly_newton_step(eccentricity, mean_anomaly, mean_anomaly, Num.infinity_f32, 0)

eccentric_anomaly_newton_step = |eccentricity, mean_anomaly, eccentric_anomaly, error, iteration_count|
    tolerance = 1e-4
    max_iterations = 100

    if error > tolerance and iteration_count < max_iterations then
        new_eccentric_anomaly =
            eccentric_anomaly
            -
            kepler_equation(eccentricity, mean_anomaly, eccentric_anomaly)
            / kepler_equation_deriv(eccentricity, eccentric_anomaly)

        new_error = Num.abs(new_eccentric_anomaly - eccentric_anomaly)

        eccentric_anomaly_newton_step(
            eccentricity,
            mean_anomaly,
            new_eccentric_anomaly,
            new_error,
            iteration_count + 1,
        )
    else
        eccentric_anomaly

kepler_equation = |eccentricity, mean_anomaly, eccentric_anomaly|
    eccentric_anomaly - eccentricity * Num.sin(eccentric_anomaly) - mean_anomaly

kepler_equation_deriv = |eccentricity, eccentric_anomaly|
    1.0 - eccentricity * Num.cos(eccentric_anomaly)

compute_cos_true_anomaly_and_true_anomaly_per_eccentric_anomaly = |eccentricity, eccentric_anomaly|
    squared_eccentricity_factor = (1.0 + eccentricity) / (1.0 - eccentricity)
    eccentricity_factor = Num.sqrt(squared_eccentricity_factor)

    squared_tan_half_eccentric_anomaly = Num.tan(0.5 * eccentric_anomaly) |> Num.pow(2)

    squared_tan_half_true_anomaly = squared_eccentricity_factor * squared_tan_half_eccentric_anomaly

    one_over_one_plus_squared_tan_half_true_anomaly = 1.0 / (1.0 + squared_tan_half_true_anomaly)

    cos_true_anomaly = (1.0 - squared_tan_half_true_anomaly) * one_over_one_plus_squared_tan_half_true_anomaly

    true_anomaly_per_eccentric_anomaly =
        eccentricity_factor
        * (1.0 + squared_tan_half_eccentric_anomaly)
        * one_over_one_plus_squared_tan_half_true_anomaly

    (cos_true_anomaly, true_anomaly_per_eccentric_anomaly)

compute_orbital_distance = |semi_major_axis, eccentricity, cos_true_anomaly|
    semi_major_axis * (1.0 - Num.pow(eccentricity, 2)) / (1.0 + eccentricity * cos_true_anomaly)

compute_sin_true_anomaly = |eccentric_anomaly, cos_true_anomaly|
    sin_true_anomaly = Num.sqrt(1.0 - Num.pow(cos_true_anomaly, 2))
    if eccentric_anomaly <= Num.pi then
        sin_true_anomaly
    else
        Num.neg(sin_true_anomaly)

compute_orbital_displacement = |cos_true_anomaly, sin_true_anomaly, orbital_distance|
    (
        orbital_distance * cos_true_anomaly,
        orbital_distance * sin_true_anomaly,
        0.0,
    )

compute_rate_of_change_of_true_anomaly = |eccentricity, mean_angular_speed, eccentric_anomaly, true_anomaly_per_eccentric_anomaly|
    mean_angular_speed
    * true_anomaly_per_eccentric_anomaly
    / (1.0 - eccentricity * Num.cos(eccentric_anomaly))

compute_radial_speed = |semi_major_axis, eccentricity, cos_true_anomaly, sin_true_anomaly, rate_of_change_of_true_anomaly|
    rate_of_change_of_true_anomaly
    * eccentricity
    * semi_major_axis
    * (1.0 - Num.pow(eccentricity, 2))
    * sin_true_anomaly
    / (
        (1.0 + eccentricity * cos_true_anomaly)
        |> Num.pow(2)
    )

compute_tangential_speed = |orbital_distance, rate_of_change_of_true_anomaly|
    orbital_distance * rate_of_change_of_true_anomaly

compute_orbital_velocity = |cos_true_anomaly, sin_true_anomaly, radial_speed, tangential_speed|
    (
        radial_speed * cos_true_anomaly - tangential_speed * sin_true_anomaly,
        radial_speed * sin_true_anomaly + tangential_speed * cos_true_anomaly,
        0.0,
    )
