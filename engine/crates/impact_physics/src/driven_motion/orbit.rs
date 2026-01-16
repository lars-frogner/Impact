//! Orbital motion.

use crate::{
    driven_motion::MotionDriverRegistry,
    quantities::{OrientationC, Position, PositionC, Velocity, VelocityC},
    rigid_body::{KinematicRigidBodyID, RigidBodyManager},
};
use approx::abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_math::consts::f32::{PI, TWO_PI};
use roc_integration::roc;

/// Manages all [`OrbitalTrajectoryDriver`]s.
pub type OrbitalTrajectoryRegistry =
    MotionDriverRegistry<OrbitalTrajectoryDriverID, OrbitalTrajectoryDriver>;

define_component_type! {
    /// Identifier for a [`OrbitalTrajectoryDriver`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct OrbitalTrajectoryDriverID(u64);
}

/// Driver for imposing an orbital trajectory on a kinematic rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct OrbitalTrajectoryDriver {
    /// The kinematic rigid body being driven.
    pub rigid_body_id: KinematicRigidBodyID,
    /// The orbital trajectory imposed on the body.
    pub trajectory: OrbitalTrajectory,
    padding: f32,
}

define_setup_type! {
    /// An orbital trajectory.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct OrbitalTrajectory {
        /// When (in simulation time) the orbiting body should be at the periapsis
        /// (the closest point to the orbited body).
        pub periapsis_time: f32,
        /// The orientation of the orbit. The first axis of the oriented orbit frame
        /// will coincide with the direction from the orbited body to the periapsis,
        /// the second with the direction of the velocity at the periapsis and the
        /// third with the normal of the orbital plane.
        pub orientation: OrientationC,
        /// The position of the focal point where the body being orbited would be
        /// located.
        pub focal_position: PositionC,
        /// Half the longest diameter of the orbital ellipse.
        pub semi_major_axis: f32,
        /// The eccentricity of the orbital ellipse (0 is circular, 1 is a line).
        pub eccentricity: f32,
        /// The orbital period.
        pub period: f32,
    }
}

impl From<u64> for OrbitalTrajectoryDriverID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl OrbitalTrajectoryDriver {
    pub fn new(rigid_body_id: KinematicRigidBodyID, trajectory: OrbitalTrajectory) -> Self {
        Self {
            rigid_body_id,
            trajectory,
            padding: 0.0,
        }
    }

    /// Resets the appropriate properties of the driven rigid body in
    /// preparation for applying driven properties.
    pub fn reset(&self, rigid_body_manager: &mut RigidBodyManager) {
        let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(self.rigid_body_id)
        else {
            return;
        };
        rigid_body.set_position(PositionC::origin());
        rigid_body.set_velocity(VelocityC::zeros());
    }

    /// Applies the driven properties for the given time to the appropriate
    /// rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, time: f32) {
        let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(self.rigid_body_id)
        else {
            return;
        };

        let (trajectory_position, trajectory_velocity) =
            self.trajectory.compute_position_and_velocity(time);

        rigid_body.set_position(rigid_body.position() + trajectory_position.as_vector());
        rigid_body.set_velocity(rigid_body.velocity() + trajectory_velocity);
    }
}

#[roc]
impl OrbitalTrajectory {
    /// Creates a new orbital trajectory with the given properties.
    #[roc(body = r#"
    {
        periapsis_time,
        orientation,
        focal_position,
        semi_major_axis,
        eccentricity,
        period,
    }
    "#)]
    pub fn new(
        periapsis_time: f32,
        orientation: OrientationC,
        focal_position: PositionC,
        semi_major_axis: f32,
        eccentricity: f32,
        period: f32,
    ) -> Self {
        Self {
            periapsis_time,
            orientation,
            focal_position,
            semi_major_axis,
            eccentricity,
            period,
        }
    }

    /// Computes the position and velocity for the trajectory at the given time.
    ///
    /// # Panics
    /// - If the semi-major axis does not exceed zero.
    /// - If the eccentricity is not between zero (inclusive) and one
    ///   (exclusive).
    /// - If the period is zero.
    pub fn compute_position_and_velocity(&self, time: f32) -> (PositionC, VelocityC) {
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

        let orientation = self.orientation.aligned();
        let focal_position = self.focal_position.aligned();

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

        let world_space_orbital_displacement = orientation.rotate_point(&orbital_displacement);

        let world_space_orbital_position =
            focal_position + world_space_orbital_displacement.as_vector();

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

        let world_space_orbital_velocity = orientation.rotate_vector(&orbital_velocity);

        (
            world_space_orbital_position.compact(),
            world_space_orbital_velocity.compact(),
        )
    }

    fn compute_mean_angular_speed(period: f32) -> f32 {
        TWO_PI / period
    }

    fn compute_mean_anomaly(periapsis_time: f32, mean_angular_speed: f32, time: f32) -> f32 {
        mean_angular_speed * (time - periapsis_time) % TWO_PI
    }

    fn compute_eccentric_anomaly(eccentricity: f32, mean_anomaly: f32) -> f32 {
        const TOLERANCE: f32 = 1e-4;
        const MAX_ITERATIONS: usize = 100;

        let kepler_equation = |eccentric_anomaly| {
            eccentric_anomaly - eccentricity * f32::sin(eccentric_anomaly) - mean_anomaly
        };

        let kepler_equation_deriv =
            |eccentric_anomaly| 1.0 - eccentricity * f32::cos(eccentric_anomaly);

        let mut eccentric_anomaly = mean_anomaly;
        let mut error = f32::INFINITY;
        let mut iteration_count = 0;

        while error > TOLERANCE && iteration_count < MAX_ITERATIONS {
            let new_eccentric_anomaly = eccentric_anomaly
                - kepler_equation(eccentric_anomaly) / kepler_equation_deriv(eccentric_anomaly);

            error = (new_eccentric_anomaly - eccentric_anomaly).abs();
            eccentric_anomaly = new_eccentric_anomaly;

            iteration_count += 1;
        }

        eccentric_anomaly
    }

    fn compute_cos_true_anomaly_and_true_anomaly_per_eccentric_anomaly(
        eccentricity: f32,
        eccentric_anomaly: f32,
    ) -> (f32, f32) {
        let squared_eccentricity_factor = (1.0 + eccentricity) / (1.0 - eccentricity);
        let eccentricity_factor = f32::sqrt(squared_eccentricity_factor);

        let squared_tan_half_eccentric_anomaly = f32::tan(0.5 * eccentric_anomaly).powi(2);

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
        semi_major_axis: f32,
        eccentricity: f32,
        cos_true_anomaly: f32,
    ) -> f32 {
        semi_major_axis * (1.0 - eccentricity.powi(2)) / (1.0 + eccentricity * cos_true_anomaly)
    }

    fn compute_sin_true_anomaly(eccentric_anomaly: f32, cos_true_anomaly: f32) -> f32 {
        let sin_true_anomaly = f32::sqrt(1.0 - cos_true_anomaly.powi(2));
        if eccentric_anomaly <= PI {
            sin_true_anomaly
        } else {
            -sin_true_anomaly
        }
    }

    fn compute_orbital_displacement(
        cos_true_anomaly: f32,
        sin_true_anomaly: f32,
        orbital_distance: f32,
    ) -> Position {
        Position::new(
            orbital_distance * cos_true_anomaly,
            orbital_distance * sin_true_anomaly,
            0.0,
        )
    }

    fn compute_rate_of_change_of_true_anomaly(
        eccentricity: f32,
        mean_angular_speed: f32,
        eccentric_anomaly: f32,
        true_anomaly_per_eccentric_anomaly: f32,
    ) -> f32 {
        mean_angular_speed * true_anomaly_per_eccentric_anomaly
            / (1.0 - eccentricity * f32::cos(eccentric_anomaly))
    }

    fn compute_radial_speed(
        semi_major_axis: f32,
        eccentricity: f32,
        cos_true_anomaly: f32,
        sin_true_anomaly: f32,
        rate_of_change_of_true_anomaly: f32,
    ) -> f32 {
        rate_of_change_of_true_anomaly
            * eccentricity
            * semi_major_axis
            * (1.0 - eccentricity.powi(2))
            * sin_true_anomaly
            / (1.0 + eccentricity * cos_true_anomaly).powi(2)
    }

    fn compute_tangential_speed(orbital_distance: f32, rate_of_change_of_true_anomaly: f32) -> f32 {
        orbital_distance * rate_of_change_of_true_anomaly
    }

    fn compute_orbital_velocity(
        cos_true_anomaly: f32,
        sin_true_anomaly: f32,
        radial_speed: f32,
        tangential_speed: f32,
    ) -> Velocity {
        Velocity::new(
            radial_speed * cos_true_anomaly - tangential_speed * sin_true_anomaly,
            radial_speed * sin_true_anomaly + tangential_speed * cos_true_anomaly,
            0.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantities::{DirectionC, Orientation, OrientationC};
    use approx::abs_diff_eq;
    use impact_math::{consts::f32::FRAC_PI_2, point::Point3, vector::UnitVector3C};
    use proptest::prelude::*;

    prop_compose! {
        fn position_strategy(max_position_coord: f32)(
            position_coord_x in -max_position_coord..max_position_coord,
            position_coord_y in -max_position_coord..max_position_coord,
            position_coord_z in -max_position_coord..max_position_coord,
        ) -> PositionC {
            PositionC::new(position_coord_x, position_coord_y, position_coord_z)
        }
    }

    prop_compose! {
        fn direction_strategy()(
            phi in 0.0..TWO_PI,
            theta in 0.0..PI,
        ) -> DirectionC {
            DirectionC::new_unchecked(
                f32::cos(phi) * f32::sin(theta),
                f32::sin(phi) * f32::sin(theta),
                f32::cos(theta)
            )
        }
    }

    prop_compose! {
        fn orientation_strategy()(
            rotation_y in 0.0..TWO_PI,
            rotation_x in -FRAC_PI_2..FRAC_PI_2,
            rotation_z in 0.0..TWO_PI,
        ) -> OrientationC {
            Orientation::from_euler_angles_extrinsic(rotation_y, rotation_x, rotation_z).compact()
        }
    }

    #[test]
    #[should_panic]
    fn should_panic_if_semi_major_axis_is_zero() {
        let trajectory = OrbitalTrajectory::new(
            0.0,
            OrientationC::identity(),
            PositionC::origin(),
            0.0,
            0.0,
            1.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_semi_major_axis_is_negative() {
        let trajectory = OrbitalTrajectory::new(
            0.0,
            OrientationC::identity(),
            PositionC::origin(),
            -0.1,
            0.0,
            1.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_eccentricity_is_negative() {
        let trajectory = OrbitalTrajectory::new(
            0.0,
            OrientationC::identity(),
            PositionC::origin(),
            1.0,
            -0.1,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_eccentricity_is_one() {
        let trajectory = OrbitalTrajectory::new(
            0.0,
            OrientationC::identity(),
            PositionC::origin(),
            1.0,
            1.0,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_period_is_zero() {
        let trajectory = OrbitalTrajectory::new(
            0.0,
            OrientationC::identity(),
            PositionC::origin(),
            1.0,
            0.0,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    proptest! {
        #[test]
        fn should_get_periapsis_and_apoapsis_position_at_whole_and_half_periods_from_periapsis_time(
            periapsis_time in -1e1..1e1_f32,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2_f32,
            eccentricity in 0.0_f32..0.9,
            period in 1e-1..1e2_f32,
            n_periods in 0..20,
        ) {
            let trajectory = OrbitalTrajectory::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + n_periods as f32 * period;
            let apoapsis_time = periapsis_time + 0.5 * period;

            let periapsis_distance =
                semi_major_axis * (1.0 - eccentricity.powi(2)) / (1.0 + eccentricity);
            let apoapsis_distance =
                semi_major_axis * (1.0 - eccentricity.powi(2)) / (1.0 - eccentricity);

            let correct_periapsis_position = focal_position
                + orientation.aligned().rotate_point(&(Point3::new(periapsis_distance, 0.0, 0.0))).as_vector().compact();

            let correct_apoapsis_position = focal_position
                + orientation.aligned().rotate_point(&(Point3::new(-apoapsis_distance, 0.0, 0.0))).as_vector().compact();

            let periapsis_position = trajectory.compute_position_and_velocity(periapsis_time).0;
            let apoapsis_position = trajectory.compute_position_and_velocity(apoapsis_time).0;

            prop_assert!(abs_diff_eq!(
                periapsis_position,
                correct_periapsis_position,
                epsilon = 1e-3 * semi_major_axis
            ));
            prop_assert!(abs_diff_eq!(
                apoapsis_position,
                correct_apoapsis_position,
                epsilon = 1e-3 * semi_major_axis
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_velocities_normal_to_displacement_at_periapsis_and_apoapsis(
            periapsis_time in -1e1..1e1_f32,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2_f32,
            eccentricity in 0.0_f32..0.9,
            period in 1e-1..1e2_f32,
            n_periods in 0..20,
        ) {
            let trajectory = OrbitalTrajectory::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + n_periods as f32 * period;
            let apoapsis_time = periapsis_time + 0.5 * period;

            let (periapsis_position, periapsis_velocity) =
                trajectory.compute_position_and_velocity(periapsis_time);
            let (apoapsis_position, apoapsis_velocity) =
                trajectory.compute_position_and_velocity(apoapsis_time);

            let periapsis_displacement = periapsis_position - focal_position;
            let apoapsis_displacement = apoapsis_position - focal_position;

            prop_assert!(abs_diff_eq!(
                UnitVector3C::normalized_from(periapsis_velocity).dot(&UnitVector3C::normalized_from(periapsis_displacement)),
                0.0,
                epsilon = 1e-2
            ));
            prop_assert!(abs_diff_eq!(
               UnitVector3C::normalized_from(apoapsis_velocity).dot(&UnitVector3C::normalized_from(apoapsis_displacement)),
                0.0,
                epsilon = 1e-2
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_higher_periapsis_than_apoapsis_speed(
            periapsis_time in -1e1..1e1_f32,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2_f32,
            eccentricity in 0.0_f32..0.9,
            period in 1e-1..1e2_f32,
            n_periods in 0..20,
        ) {
            let trajectory = OrbitalTrajectory::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + n_periods as f32 * period;
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
            periapsis_time in -1e1..1e1_f32,
            orientation in orientation_strategy(),
            focal_position in position_strategy(1e2),
            semi_major_axis in 1e-2..1e2_f32,
            eccentricity in 0.0_f32..0.9,
            period in 1e-1..1e2_f32,
            n_periods in 0..20,
        ) {
            let trajectory = OrbitalTrajectory::new(
                periapsis_time,
                orientation,
                focal_position,
                semi_major_axis,
                eccentricity,
                period,
            );
            let periapsis_time = periapsis_time + n_periods as f32 * period;
            let apoapsis_time = periapsis_time + 0.5 * period;

            let periapsis_velocity_direction = trajectory
                .compute_position_and_velocity(periapsis_time)
                .1
                .normalized();
            let apoapsis_velocity_direction = trajectory
                .compute_position_and_velocity(apoapsis_time)
                .1
                .normalized();

            prop_assert!(abs_diff_eq!(
                UnitVector3C::normalized_from(periapsis_velocity_direction).dot(&UnitVector3C::normalized_from(apoapsis_velocity_direction)),
                -1.0,
                epsilon = 1e-2
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_circular_position_and_velocity_with_zero_eccentricity(
            periapsis_time in -1e1..1e1_f32,
            orientation in orientation_strategy(),
            center_position in position_strategy(1e2),
            radius in 1e-2..1e2_f32,
            period in 1e-1..1e2_f32,
            time in -1e1..1e1_f32,
        ) {
            let trajectory = OrbitalTrajectory::new(
                periapsis_time,
                orientation,
                center_position,
                radius,
                0.0,
                period,
            );

            let (position, velocity) = trajectory.compute_position_and_velocity(time);
            let displacement = position - center_position;

            prop_assert!(abs_diff_eq!(displacement.norm(), radius, epsilon = 1e-3 * radius));
            prop_assert!(abs_diff_eq!(
                velocity.norm(),
                TWO_PI * radius / period,
                epsilon = 1e-3 * radius / period
            ));
            prop_assert!(abs_diff_eq!(
                UnitVector3C::normalized_from(velocity).dot(&UnitVector3C::normalized_from(displacement)),
                0.0,
                epsilon = 1e-3
            ));
        }
    }
}
