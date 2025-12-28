//! Harmonic oscillation.

use crate::{
    driven_motion::MotionDriverRegistry,
    quantities::{DirectionP, PositionP, VelocityP},
    rigid_body::{KinematicRigidBodyID, RigidBodyManager},
};
use approx::abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_math::consts::f32::TWO_PI;
use roc_integration::roc;

/// Manages all [`HarmonicOscillatorTrajectoryDriver`]s.
pub type HarmonicOscillatorTrajectoryRegistry =
    MotionDriverRegistry<HarmonicOscillatorTrajectoryDriverID, HarmonicOscillatorTrajectoryDriver>;

define_component_type! {
    /// Identifier for a [`HarmonicOscillatorTrajectoryDriver`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct HarmonicOscillatorTrajectoryDriverID(u64);
}

/// Driver for imposing a harmonically oscillating trajectory on a kinematic
/// rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct HarmonicOscillatorTrajectoryDriver {
    /// The kinematic rigid body being driven.
    pub rigid_body_id: KinematicRigidBodyID,
    /// The harmonic oscillator trajectory imposed on the body.
    pub trajectory: HarmonicOscillatorTrajectory,
    padding: f32,
}

define_setup_type! {
    /// A harmonically oscillating trajectory.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HarmonicOscillatorTrajectory {
        /// A simulation time when the body should be at the center of
        /// oscillation.
        pub center_time: f32,
        /// The position of the center of oscillation.
        pub center_position: PositionP,
        /// The direction in which the body is oscillating back and forth.
        pub direction: DirectionP,
        /// The maximum distance of the body from the center position.
        pub amplitude: f32,
        /// The duration of one full oscillation.
        pub period: f32,
    }
}

impl From<u64> for HarmonicOscillatorTrajectoryDriverID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl HarmonicOscillatorTrajectoryDriver {
    pub fn new(
        rigid_body_id: KinematicRigidBodyID,
        trajectory: HarmonicOscillatorTrajectory,
    ) -> Self {
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
        rigid_body.set_position(PositionP::origin());
        rigid_body.set_velocity(VelocityP::zeros());
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
impl HarmonicOscillatorTrajectory {
    /// Creates a new harmonically oscillating trajectory with the given
    /// properties.
    #[roc(body = r#"
    {
        center_time,
        center_position,
        direction,
        amplitude,
        period,
    }
    "#)]
    pub fn new(
        center_time: f32,
        center_position: PositionP,
        direction: DirectionP,
        amplitude: f32,
        period: f32,
    ) -> Self {
        Self {
            center_time,
            center_position,
            direction,
            amplitude,
            period,
        }
    }

    /// Computes the position and velocity for the trajectory at the given time.
    ///
    /// # Panics
    /// If the period is zero.
    pub fn compute_position_and_velocity(&self, time: f32) -> (PositionP, VelocityP) {
        assert!(
            abs_diff_ne!(self.period, 0.0),
            "Period of harmonically oscillating trajectory is zero"
        );

        let center_position = self.center_position.unpack();
        let direction = self.direction.unpack();

        let center_time_offset = time - self.center_time;
        let angular_frequency = TWO_PI / self.period;

        let position = center_position
            + (self.amplitude * f32::sin(angular_frequency * center_time_offset)) * direction;

        let velocity = ((self.amplitude * angular_frequency)
            * f32::cos(angular_frequency * center_time_offset))
            * direction;

        (position.pack(), velocity.pack())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantities::DirectionP;
    use approx::abs_diff_eq;
    use impact_math::{
        consts::f32::{PI, TWO_PI},
        vector::{UnitVector3P, Vector3P},
    };
    use proptest::prelude::*;

    prop_compose! {
        fn position_strategy(max_position_coord: f32)(
            position_coord_x in -max_position_coord..max_position_coord,
            position_coord_y in -max_position_coord..max_position_coord,
            position_coord_z in -max_position_coord..max_position_coord,
        ) -> PositionP {
            PositionP::new(position_coord_x, position_coord_y, position_coord_z)
        }
    }

    prop_compose! {
        fn direction_strategy()(
            phi in 0.0..TWO_PI,
            theta in 0.0..PI,
        ) -> DirectionP {
            DirectionP::normalized_from(Vector3P::new(
                f32::cos(phi) * f32::sin(theta),
                f32::sin(phi) * f32::sin(theta),
                f32::cos(theta)
            ))
        }
    }

    #[test]
    #[should_panic]
    fn should_panic_if_period_is_zero() {
        let trajectory = HarmonicOscillatorTrajectory::new(
            0.0,
            PositionP::origin(),
            UnitVector3P::unit_x(),
            1.0,
            0.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    proptest! {
        #[test]
        fn should_get_center_position_at_half_periods_from_center_time(
            center_time in -1e1..1e1_f32,
            center_position in position_strategy(1e2),
            direction in direction_strategy(),
            amplitude in -1e2..1e2_f32,
            period in 1e-1..1e2_f32,
            n_half_periods in 0..20,
        ) {
            let trajectory = HarmonicOscillatorTrajectory::new(
                center_time,
                center_position,
                direction,
                amplitude,
                period,
            );
            let time = center_time + n_half_periods as f32 * 0.5 * period;
            let (trajectory_position, _) = trajectory.compute_position_and_velocity(time);
            prop_assert!(abs_diff_eq!(
                trajectory_position,
                center_position,
                epsilon = 1e-3 * center_position.as_vector().unpack().component_abs().max_component()
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_peak_position_and_zero_velocity_at_quarter_periods_from_center_time(
            center_time in -1e1..1e1_f32,
            center_position in position_strategy(1e2),
            direction in direction_strategy(),
            amplitude in -1e2..1e2_f32,
            period in 1e-1..1e2_f32,
            n_periods in 0..20,
        ) {
            let trajectory = HarmonicOscillatorTrajectory::new(
                center_time,
                center_position,
                direction,
                amplitude,
                period,
            );
            let center_time = center_time + n_periods as f32 * period;
            let positive_peak_time = center_time + 0.25 * period;
            let negative_peak_time = center_time - 0.25 * period;

            let positive_peak_position = center_position + amplitude * direction;
            let negative_peak_position = center_position - amplitude * direction;

            let (
                positive_peak_trajectory_position,
                positive_peak_trajectory_velocity,
            ) = trajectory.compute_position_and_velocity(positive_peak_time);
            let (
                negative_peak_trajectory_position,
                negative_peak_trajectory_velocity,
            ) = trajectory.compute_position_and_velocity(negative_peak_time);

            prop_assert!(abs_diff_eq!(
                positive_peak_trajectory_position,
                positive_peak_position,
                epsilon = 1e-3 * positive_peak_position.as_vector().unpack().component_abs().max_component()
            ));
            prop_assert!(abs_diff_eq!(positive_peak_trajectory_velocity, VelocityP::zeros(), epsilon = 5e-1));
            prop_assert!(abs_diff_eq!(
                negative_peak_trajectory_position,
                negative_peak_position,
                epsilon = 1e-3 * negative_peak_position.as_vector().unpack().component_abs().max_component()
            ));
            prop_assert!(abs_diff_eq!(negative_peak_trajectory_velocity, VelocityP::zeros(), epsilon = 5e-1));
        }
    }
}
