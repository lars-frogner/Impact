//! Circular motion.

pub mod components;

use crate::{
    num::Float,
    physics::{
        fph,
        motion::{Position, Velocity},
    },
};
use approx::abs_diff_ne;
use components::CircularTrajectoryComp;
use nalgebra::{point, vector};

impl CircularTrajectoryComp {
    /// Computes the position and velocity for the trajectory at the given time.
    ///
    /// # Panics
    /// - If the radius does not exceed zero.
    /// - If the period is zero.
    pub fn compute_position_and_velocity(&self, time: fph) -> (Position, Velocity) {
        assert!(
            self.radius > 0.0,
            "Radius of circular trajectory does not exceed zero"
        );
        assert!(
            abs_diff_ne!(self.period, 0.0),
            "Period of circular trajectory is zero"
        );

        let angular_speed = Self::compute_angular_speed(self.period);

        let angle = Self::compute_angle(self.initial_time, angular_speed, time);
        let (sin_angle, cos_angle) = angle.sin_cos();

        let circular_displacement =
            Self::compute_circular_displacement(self.radius, cos_angle, sin_angle);

        let world_space_circular_displacement =
            self.orientation.transform_point(&circular_displacement);

        let world_space_circular_position =
            self.center_position + world_space_circular_displacement.coords;

        let tangential_speed = self.radius * angular_speed;

        let circular_velocity =
            Self::compute_circular_velocity(cos_angle, sin_angle, tangential_speed);

        let world_space_circular_velocity = self.orientation.transform_vector(&circular_velocity);

        (world_space_circular_position, world_space_circular_velocity)
    }

    fn compute_angular_speed(period: fph) -> fph {
        fph::TWO_PI / period
    }

    fn compute_angle(initial_time: fph, angular_speed: fph, time: fph) -> fph {
        angular_speed * (time - initial_time) % fph::TWO_PI
    }

    fn compute_circular_displacement(radius: fph, cos_angle: fph, sin_angle: fph) -> Position {
        point![radius * cos_angle, radius * sin_angle, 0.0]
    }

    fn compute_circular_velocity(
        cos_angle: fph,
        sin_angle: fph,
        tangential_speed: fph,
    ) -> Velocity {
        vector![
            -tangential_speed * sin_angle,
            tangential_speed * cos_angle,
            0.0
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        num::Float,
        physics::motion::{Direction, Orientation},
    };
    use approx::abs_diff_eq;
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
    fn should_panic_if_radius_is_zero() {
        let trajectory =
            CircularTrajectoryComp::new(0.0, Orientation::identity(), Position::origin(), 0.0, 1.0);
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_radius_is_negative() {
        let trajectory = CircularTrajectoryComp::new(
            0.0,
            Orientation::identity(),
            Position::origin(),
            -0.1,
            1.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_period_is_zero() {
        let trajectory =
            CircularTrajectoryComp::new(0.0, Orientation::identity(), Position::origin(), 1.0, 0.0);
        trajectory.compute_position_and_velocity(1.0);
    }

    proptest! {
        #[test]
        fn should_get_antiparallel_velocities_at_half_period_offset(
            initial_time in -1e2..1e2,
            orientation in orientation_strategy(),
            center_position in position_strategy(1e2),
            radius in 1e-2..1e2,
            period in 1e-2..1e2,
            time in -1e2..1e2,
        ) {
            let trajectory = CircularTrajectoryComp::new(
                initial_time,
                orientation,
                center_position,
                radius,
                period,
            );
            let half_period_offset_time = time + 0.5 * period;

            let first_velocity_direction = trajectory
                .compute_position_and_velocity(time)
                .1
                .normalize();
            let second_velocity_direction = trajectory
                .compute_position_and_velocity(half_period_offset_time)
                .1
                .normalize();

            prop_assert!(abs_diff_eq!(
                first_velocity_direction.dot(&second_velocity_direction),
                -1.0,
                epsilon = 1e-6
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_circular_position_and_velocity(
            initial_time in -1e2..1e2,
            orientation in orientation_strategy(),
            center_position in position_strategy(1e2),
            radius in 1e-2..1e2,
            period in 1e-2..1e2,
            time in -1e2..1e2,
        ) {
            let trajectory = CircularTrajectoryComp::new(
                initial_time,
                orientation,
                center_position,
                radius,
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
