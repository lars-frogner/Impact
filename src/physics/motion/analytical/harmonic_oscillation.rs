//! Harmonically oscillating trajectories.

pub mod components;

use crate::{
    num::Float,
    physics::{
        fph,
        motion::{Position, Velocity},
    },
};
use approx::abs_diff_ne;
use components::HarmonicOscillatorTrajectoryComp;

impl HarmonicOscillatorTrajectoryComp {
    /// Computes the position and velocity for the trajectory at the given time.
    ///
    /// # Panics
    /// If the period is zero.
    pub fn compute_position_and_velocity(&self, time: fph) -> (Position, Velocity) {
        assert!(
            abs_diff_ne!(self.period, 0.0),
            "Period of harmonically oscillating trajectory is zero"
        );

        let center_time_offset = time - self.center_time;
        let angular_frequency = fph::TWO_PI / self.period;

        let position = self.center_position
            + (self.amplitude * fph::sin(angular_frequency * center_time_offset))
                * self.direction.as_ref();

        let velocity = ((self.amplitude * angular_frequency)
            * fph::cos(angular_frequency * center_time_offset))
            * self.direction.as_ref();

        (position, velocity)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{num::Float, physics::motion::Direction};
    use approx::abs_diff_eq;
    use nalgebra::{point, vector, Vector3};
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

    #[test]
    #[should_panic]
    fn should_panic_if_period_is_zero() {
        let trajectory = HarmonicOscillatorTrajectoryComp::new(
            0.0,
            Position::origin(),
            Vector3::x_axis(),
            1.0,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    proptest! {
        #[test]
        fn should_get_center_position_at_half_periods_from_center_time(
            center_time in -1e2..1e2,
            center_position in position_strategy(1e2),
            direction in direction_strategy(),
            amplitude in -1e2..1e2,
            period in 1e-2..1e2,
            n_half_periods in 0..100,
        ) {
            let trajectory = HarmonicOscillatorTrajectoryComp::new(
                center_time,
                center_position,
                direction,
                amplitude,
                period,
            );
            let time = center_time + fph::from(n_half_periods) * 0.5 * period;
            let (trajectory_position, _) = trajectory.compute_position_and_velocity(time);
            prop_assert!(abs_diff_eq!(trajectory_position, center_position, epsilon = 1e-6));
        }
    }

    proptest! {
        #[test]
        fn should_get_peak_position_and_zero_velocity_at_quarter_periods_from_center_time(
            center_time in -1e2..1e2,
            center_position in position_strategy(1e2),
            direction in direction_strategy(),
            amplitude in -1e2..1e2,
            period in 1e-2..1e2,
            n_periods in 0..100,
        ) {
            let trajectory = HarmonicOscillatorTrajectoryComp::new(
                center_time,
                center_position,
                direction,
                amplitude,
                period,
            );
            let center_time = center_time + fph::from(n_periods) * period;
            let positive_peak_time = center_time + 0.25 * period;
            let negative_peak_time = center_time - 0.25 * period;

            let positive_peak_position = center_position + amplitude * direction.as_ref();
            let negative_peak_position = center_position - amplitude * direction.as_ref();

            let (
                positive_peak_trajectory_position,
                positive_peak_trajectory_velocity,
            ) = trajectory.compute_position_and_velocity(positive_peak_time);
            let (
                negative_peak_trajectory_position,
                negative_peak_trajectory_velocity,
            ) = trajectory.compute_position_and_velocity(negative_peak_time);

            prop_assert!(abs_diff_eq!(positive_peak_trajectory_position, positive_peak_position, epsilon = 1e-6));
            prop_assert!(abs_diff_eq!(positive_peak_trajectory_velocity, Velocity::zeros(), epsilon = 1e-6));
            prop_assert!(abs_diff_eq!(negative_peak_trajectory_position, negative_peak_position, epsilon = 1e-6));
            prop_assert!(abs_diff_eq!(negative_peak_trajectory_velocity, Velocity::zeros(), epsilon = 1e-6));
        }
    }
}
