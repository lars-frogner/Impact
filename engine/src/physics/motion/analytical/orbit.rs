//! Orbital motion.

pub mod components;

use crate::physics::{
    fph,
    motion::{Position, Velocity},
};
use approx::abs_diff_ne;
use components::OrbitalTrajectoryComp;
use impact_math::Float;
use nalgebra::{point, vector};
use roots::{self, SimpleConvergency};

impl OrbitalTrajectoryComp {
    const ECCENTRIC_ANOMALY_CONVERGENCY: SimpleConvergency<fph> = SimpleConvergency {
        eps: 1e-6,
        max_iter: 100,
    };

    /// Computes the position and velocity for the trajectory at the given time.
    ///
    /// # Panics
    /// - If the semi-major axis does not exceed zero.
    /// - If the eccentricity is not between zero (inclusive) and one
    ///   (exclusive).
    /// - If the period is zero.
    pub fn compute_position_and_velocity(&self, time: fph) -> (Position, Velocity) {
        assert!(
            self.semi_major_axis > 0.0,
            "Semi-major axis of orbital trajectory does not exceed zero"
        );
        assert!(
            self.eccentricity >= 0.0,
            "Eccentricity of orbital trajectory is negative"
        );
        assert!(
            self.eccentricity < 1.0,
            "Eccentricity of orbital trajectory is not smaller than one"
        );
        assert!(
            abs_diff_ne!(self.period, 0.0),
            "Period of orbital trajectory is zero"
        );

        let mean_angular_speed = Self::compute_mean_angular_speed(self.period);

        let mean_anomaly =
            Self::compute_mean_anomaly(self.periapsis_time, mean_angular_speed, time);

        let eccentric_anomaly = Self::compute_eccentric_anomaly(self.eccentricity, mean_anomaly);

        let (cos_true_anomaly, true_anomaly_per_eccentric_anomaly) =
            Self::compute_cos_true_anomaly_and_true_anomaly_per_eccentric_anomaly(
                self.eccentricity,
                eccentric_anomaly,
            );

        let orbital_distance = Self::compute_orbital_distance(
            self.semi_major_axis,
            self.eccentricity,
            cos_true_anomaly,
        );

        let sin_true_anomaly = Self::compute_sin_true_anomaly(eccentric_anomaly, cos_true_anomaly);

        let orbital_displacement = Self::compute_orbital_displacement(
            cos_true_anomaly,
            sin_true_anomaly,
            orbital_distance,
        );

        let world_space_orbital_displacement =
            self.orientation.transform_point(&orbital_displacement);

        let world_space_orbital_position =
            self.focal_position + world_space_orbital_displacement.coords;

        let rate_of_change_of_true_anomaly = Self::compute_rate_of_change_of_true_anomaly(
            self.eccentricity,
            mean_angular_speed,
            eccentric_anomaly,
            true_anomaly_per_eccentric_anomaly,
        );

        let radial_speed = Self::compute_radial_speed(
            self.semi_major_axis,
            self.eccentricity,
            cos_true_anomaly,
            sin_true_anomaly,
            rate_of_change_of_true_anomaly,
        );

        let tangential_speed =
            Self::compute_tangential_speed(orbital_distance, rate_of_change_of_true_anomaly);

        let orbital_velocity = Self::compute_orbital_velocity(
            cos_true_anomaly,
            sin_true_anomaly,
            radial_speed,
            tangential_speed,
        );

        let world_space_orbital_velocity = self.orientation.transform_vector(&orbital_velocity);

        (world_space_orbital_position, world_space_orbital_velocity)
    }

    fn compute_mean_angular_speed(period: fph) -> fph {
        fph::TWO_PI / period
    }

    fn compute_mean_anomaly(periapsis_time: fph, mean_angular_speed: fph, time: fph) -> fph {
        mean_angular_speed * (time - periapsis_time) % fph::TWO_PI
    }

    fn compute_eccentric_anomaly(eccentricity: fph, mean_anomaly: fph) -> fph {
        let kepler_equation = |eccentric_anomaly| {
            eccentric_anomaly - eccentricity * fph::sin(eccentric_anomaly) - mean_anomaly
        };

        let mut convergency = Self::ECCENTRIC_ANOMALY_CONVERGENCY;

        if let Ok(eccentric_anomaly) = roots::find_root_newton_raphson(
            mean_anomaly,
            kepler_equation,
            |eccentric_anomaly| 1.0 - eccentricity * fph::cos(eccentric_anomaly),
            &mut convergency,
        ) {
            eccentric_anomaly
        } else {
            let mut convergency = Self::ECCENTRIC_ANOMALY_CONVERGENCY;

            roots::find_root_regula_falsi(0.0, fph::TWO_PI, kepler_equation, &mut convergency)
                .expect("Could not solve Kepler's equation for the eccentric anomaly")
        }
    }

    fn compute_cos_true_anomaly_and_true_anomaly_per_eccentric_anomaly(
        eccentricity: fph,
        eccentric_anomaly: fph,
    ) -> (fph, fph) {
        let squared_eccentricity_factor = (1.0 + eccentricity) / (1.0 - eccentricity);
        let eccentricity_factor = fph::sqrt(squared_eccentricity_factor);

        let squared_tan_half_eccentric_anomaly = fph::tan(0.5 * eccentric_anomaly).powi(2);

        let squared_tan_half_true_anomaly =
            squared_eccentricity_factor * squared_tan_half_eccentric_anomaly;

        let one_over_one_plus_squared_tan_half_true_anomaly =
            1.0 / (1.0 + squared_tan_half_true_anomaly);

        let cos_true_anomaly =
            (1.0 - squared_tan_half_true_anomaly) * one_over_one_plus_squared_tan_half_true_anomaly;

        let true_anomaly_per_eccentric_anomaly = eccentricity_factor
            * (1.0 + squared_tan_half_eccentric_anomaly)
            * one_over_one_plus_squared_tan_half_true_anomaly;

        (cos_true_anomaly, true_anomaly_per_eccentric_anomaly)
    }

    fn compute_orbital_distance(
        semi_major_axis: fph,
        eccentricity: fph,
        cos_true_anomaly: fph,
    ) -> fph {
        semi_major_axis * (1.0 - eccentricity.powi(2)) / (1.0 + eccentricity * cos_true_anomaly)
    }

    fn compute_sin_true_anomaly(eccentric_anomaly: fph, cos_true_anomaly: fph) -> fph {
        let sin_true_anomaly = fph::sqrt(1.0 - cos_true_anomaly.powi(2));
        if eccentric_anomaly <= fph::PI {
            sin_true_anomaly
        } else {
            -sin_true_anomaly
        }
    }

    fn compute_orbital_displacement(
        cos_true_anomaly: fph,
        sin_true_anomaly: fph,
        orbital_distance: fph,
    ) -> Position {
        point![
            orbital_distance * cos_true_anomaly,
            orbital_distance * sin_true_anomaly,
            0.0
        ]
    }

    fn compute_rate_of_change_of_true_anomaly(
        eccentricity: fph,
        mean_angular_speed: fph,
        eccentric_anomaly: fph,
        true_anomaly_per_eccentric_anomaly: fph,
    ) -> fph {
        mean_angular_speed * true_anomaly_per_eccentric_anomaly
            / (1.0 - eccentricity * fph::cos(eccentric_anomaly))
    }

    fn compute_radial_speed(
        semi_major_axis: fph,
        eccentricity: fph,
        cos_true_anomaly: fph,
        sin_true_anomaly: fph,
        rate_of_change_of_true_anomaly: fph,
    ) -> fph {
        rate_of_change_of_true_anomaly
            * eccentricity
            * semi_major_axis
            * (1.0 - eccentricity.powi(2))
            * sin_true_anomaly
            / (1.0 + eccentricity * cos_true_anomaly).powi(2)
    }

    fn compute_tangential_speed(orbital_distance: fph, rate_of_change_of_true_anomaly: fph) -> fph {
        orbital_distance * rate_of_change_of_true_anomaly
    }

    fn compute_orbital_velocity(
        cos_true_anomaly: fph,
        sin_true_anomaly: fph,
        radial_speed: fph,
        tangential_speed: fph,
    ) -> Velocity {
        vector![
            radial_speed * cos_true_anomaly - tangential_speed * sin_true_anomaly,
            radial_speed * sin_true_anomaly + tangential_speed * cos_true_anomaly,
            0.0
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::motion::{Direction, Orientation};
    use approx::abs_diff_eq;
    use impact_math::Float;
    use nalgebra::{point, vector};
    use proptest::prelude::*;

    prop_compose! {
        fn position_strategy(max_position_coord: fph)(
            position_coord_x in -max_position_coord..max_position_coord,
            position_coord_y in -max_position_coord..max_position_coord,
            position_coord_z in -max_position_coord..max_position_coord,
        ) -> Position {
            point![position_coord_x, position_coord_y, position_coord_z]
        }
    }

    prop_compose! {
        fn direction_strategy()(
            phi in 0.0..fph::TWO_PI,
            theta in 0.0..fph::PI,
        ) -> Direction {
            Direction::new_normalize(vector![
                fph::cos(phi) * fph::sin(theta),
                fph::sin(phi) * fph::sin(theta),
                fph::cos(theta)
            ])
        }
    }

    prop_compose! {
        fn orientation_strategy()(
            rotation_roll in 0.0..fph::TWO_PI,
            rotation_pitch in -fph::FRAC_PI_2..fph::FRAC_PI_2,
            rotation_yaw in 0.0..fph::TWO_PI,
        ) -> Orientation {
            Orientation::from_euler_angles(rotation_roll, rotation_pitch, rotation_yaw)
        }
    }

    #[test]
    #[should_panic]
    fn should_panic_if_semi_major_axis_is_zero() {
        let trajectory = OrbitalTrajectoryComp::new(
            0.0,
            Orientation::identity(),
            Position::origin(),
            0.0,
            0.0,
            1.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_semi_major_axis_is_negative() {
        let trajectory = OrbitalTrajectoryComp::new(
            0.0,
            Orientation::identity(),
            Position::origin(),
            -0.1,
            0.0,
            1.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_eccentricity_is_negative() {
        let trajectory = OrbitalTrajectoryComp::new(
            0.0,
            Orientation::identity(),
            Position::origin(),
            1.0,
            -0.1,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_eccentricity_is_one() {
        let trajectory = OrbitalTrajectoryComp::new(
            0.0,
            Orientation::identity(),
            Position::origin(),
            1.0,
            1.0,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_period_is_zero() {
        let trajectory = OrbitalTrajectoryComp::new(
            0.0,
            Orientation::identity(),
            Position::origin(),
            1.0,
            0.0,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    proptest! {
        #[test]
        fn should_get_periapsis_and_apoapsis_position_at_whole_and_half_periods_from_periapsis_time(
            periapsis_time in -1e2..1e2,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2,
            eccentricity in 0.0..(1.0 - 1e-3),
            period in 1e-2..1e2,
            n_periods in 0..100,
        ) {
            let trajectory = OrbitalTrajectoryComp::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + fph::from(n_periods) * period;
            let apoapsis_time = periapsis_time + 0.5 * period;

            let periapsis_distance =
                semi_major_axis * (1.0 - eccentricity.powi(2)) / (1.0 + eccentricity);
            let apoapsis_distance =
                semi_major_axis * (1.0 - eccentricity.powi(2)) / (1.0 - eccentricity);

            let correct_periapsis_position = focal_position
                + orientation.transform_point(&(point![periapsis_distance, 0.0, 0.0])).coords;

            let correct_apoapsis_position = focal_position
                + orientation.transform_point(&(point![-apoapsis_distance, 0.0, 0.0])).coords;

            let periapsis_position = trajectory.compute_position_and_velocity(periapsis_time).0;
            let apoapsis_position = trajectory.compute_position_and_velocity(apoapsis_time).0;

            prop_assert!(abs_diff_eq!(
                periapsis_position,
                correct_periapsis_position,
                epsilon = 1e-6 * semi_major_axis
            ));
            prop_assert!(abs_diff_eq!(
                apoapsis_position,
                correct_apoapsis_position,
                epsilon = 1e-6 * semi_major_axis
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_velocities_normal_to_displacement_at_periapsis_and_apoapsis(
            periapsis_time in -1e2..1e2,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2,
            eccentricity in 0.0..(1.0 - 1e-3),
            period in 1e-2..1e2,
            n_periods in 0..100,
        ) {
            let trajectory = OrbitalTrajectoryComp::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + fph::from(n_periods) * period;
            let apoapsis_time = periapsis_time + 0.5 * period;

            let (periapsis_position, periapsis_velocity) =
                trajectory.compute_position_and_velocity(periapsis_time);
            let (apoapsis_position, apoapsis_velocity) =
                trajectory.compute_position_and_velocity(apoapsis_time);

            let periapsis_displacement = periapsis_position - focal_position;
            let apoapsis_displacement = apoapsis_position - focal_position;

            prop_assert!(abs_diff_eq!(
                periapsis_velocity.dot(&periapsis_displacement),
                0.0,
                epsilon = 1e-4 * semi_major_axis.powi(2) / period
            ));
            prop_assert!(abs_diff_eq!(
                apoapsis_velocity.dot(&apoapsis_displacement),
                0.0,
                epsilon = 1e-4 * semi_major_axis.powi(2) / period
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_higher_periapsis_than_apoapsis_speed(
            periapsis_time in -1e2..1e2,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2,
            eccentricity in 0.0..(1.0 - 1e-3),
            period in 1e-2..1e2,
            n_periods in 0..100,
        ) {
            let trajectory = OrbitalTrajectoryComp::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + fph::from(n_periods) * period;
            let apoapsis_time = periapsis_time + 0.5 * period;

            let periapsis_speed = trajectory
                .compute_position_and_velocity(periapsis_time)
                .1
                .norm();
            let apoapsis_speed = trajectory
                .compute_position_and_velocity(apoapsis_time)
                .1
                .norm();

            let speed_tolerance = 1e-9 * (periapsis_speed + apoapsis_speed);
            prop_assert!(periapsis_speed + speed_tolerance >= apoapsis_speed);
        }
    }

    proptest! {
        #[test]
        fn should_get_antiparallel_velocities_at_periapsis_and_apoapsis(
            periapsis_time in -1e2..1e2,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2,
            eccentricity in 0.0..(1.0 - 1e-3),
            period in 1e-2..1e2,
            n_periods in 0..100,
        ) {
            let trajectory = OrbitalTrajectoryComp::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + fph::from(n_periods) * period;
            let apoapsis_time = periapsis_time + 0.5 * period;

            let periapsis_velocity_direction = trajectory
                .compute_position_and_velocity(periapsis_time)
                .1
                .normalize();
            let apoapsis_velocity_direction = trajectory
                .compute_position_and_velocity(apoapsis_time)
                .1
                .normalize();

            prop_assert!(abs_diff_eq!(
                periapsis_velocity_direction.dot(&apoapsis_velocity_direction),
                -1.0,
                epsilon = 1e-5
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_circular_position_and_velocity_with_zero_eccentricity(
            periapsis_time in -1e2..1e2,
            orientation in orientation_strategy(),
            center_position in position_strategy(1e2),
            radius in 1e-2..1e2,
            period in 1e-2..1e2,
            time in -1e2..1e2,
        ) {
            let trajectory = OrbitalTrajectoryComp::new(
                periapsis_time,
                orientation,
                center_position,
                radius,
                0.0,
                period,
            );

            let (position, velocity) = trajectory.compute_position_and_velocity(time);
            let displacement = position - center_position;

            prop_assert!(abs_diff_eq!(displacement.norm(), radius, epsilon = 1e-7 * radius));
            prop_assert!(abs_diff_eq!(
                velocity.norm(),
                fph::TWO_PI * radius / period,
                epsilon = 1e-6 * radius / period
            ));
            prop_assert!(abs_diff_eq!(
                velocity.dot(&displacement),
                0.0,
                epsilon = 1e-6
            ));
        }
    }
}
