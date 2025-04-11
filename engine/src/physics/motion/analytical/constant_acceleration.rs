//! Motion with constant acceleration.

pub mod components;

use crate::physics::{
    fph,
    motion::{Position, Velocity},
};
use components::ConstantAccelerationTrajectoryComp;

impl ConstantAccelerationTrajectoryComp {
    /// Computes the position and velocity for the trajectory at the given time.
    pub fn compute_position_and_velocity(&self, time: fph) -> (Position, Velocity) {
        let time_offset = time - self.initial_time;

        let position = self.initial_position
            + time_offset * self.initial_velocity
            + (0.5 * time_offset.powi(2)) * self.acceleration;

        let velocity = self.initial_velocity + time_offset * self.acceleration;

        (position, velocity)
    }
}

#[cfg(test)]
mod tests {
    use crate::physics::motion::Acceleration;

    use super::*;
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
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
        fn velocity_strategy(max_velocity_coord: fph)(
            velocity_coord_x in -max_velocity_coord..max_velocity_coord,
            velocity_coord_y in -max_velocity_coord..max_velocity_coord,
            velocity_coord_z in -max_velocity_coord..max_velocity_coord,
        ) -> Velocity {
            vector![velocity_coord_x, velocity_coord_y, velocity_coord_z]
        }
    }

    prop_compose! {
        fn acceleration_strategy(max_acceleration_coord: fph)(
            acceleration_coord_x in -max_acceleration_coord..max_acceleration_coord,
            acceleration_coord_y in -max_acceleration_coord..max_acceleration_coord,
            acceleration_coord_z in -max_acceleration_coord..max_acceleration_coord,
        ) -> Acceleration {
            vector![acceleration_coord_x, acceleration_coord_y, acceleration_coord_z]
        }
    }

    #[test]
    fn should_get_initial_position_for_zero_velocty_and_acceleration() {
        let position = Position::origin();
        let velocity = Velocity::zeros();
        let trajectory =
            ConstantAccelerationTrajectoryComp::with_constant_velocity(0.0, position, velocity);
        let (trajectory_position, trajectory_velocity) =
            trajectory.compute_position_and_velocity(1.0);
        assert_abs_diff_eq!(trajectory_position, position);
        assert_abs_diff_eq!(trajectory_velocity, velocity);
    }

    #[test]
    fn should_get_different_position_and_velocity_for_nonzero_velocty_and_acceleration() {
        let initial_position = Position::origin();
        let initial_velocity = Velocity::y();
        let acceleration = Acceleration::z();
        let trajectory = ConstantAccelerationTrajectoryComp::new(
            0.0,
            initial_position,
            initial_velocity,
            acceleration,
        );
        let (trajectory_position, trajectory_velocity) =
            trajectory.compute_position_and_velocity(1.0);
        assert_abs_diff_ne!(trajectory_position, initial_position);
        assert_abs_diff_ne!(trajectory_velocity, initial_velocity);
    }

    proptest! {
        #[test]
        fn should_get_initial_position_and_velocity_at_initial_time(
            time in -1e2..1e2,
            position in position_strategy(1e3),
            velocity in velocity_strategy(1e3),
            acceleration in acceleration_strategy(1e2),
        ) {
            let trajectory = ConstantAccelerationTrajectoryComp::new(
                time,
                position,
                velocity,
                acceleration,
            );
            let (trajectory_position, trajectory_velocity) = trajectory.compute_position_and_velocity(time);
            prop_assert!(abs_diff_eq!(trajectory_position, position));
            prop_assert!(abs_diff_eq!(trajectory_velocity, velocity));
        }
    }
}
