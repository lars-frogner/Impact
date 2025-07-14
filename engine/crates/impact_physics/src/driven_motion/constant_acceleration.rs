//! Constant acceleration.

use crate::{
    driven_motion::MotionDriverRegistry,
    fph,
    quantities::{Acceleration, Position, Velocity},
    rigid_body::{KinematicRigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

/// Manages all [`ConstantAccelerationTrajectoryDriver`]s.
pub type ConstantAccelerationTrajectoryRegistry = MotionDriverRegistry<
    ConstantAccelerationTrajectoryDriverID,
    ConstantAccelerationTrajectoryDriver,
>;

define_component_type! {
    /// Identifier for a [`ConstantAccelerationTrajectoryDriver`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct ConstantAccelerationTrajectoryDriverID(u64);
}

/// Driver for imposing a constant acceleration trajectory on a kinematic
/// rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ConstantAccelerationTrajectoryDriver {
    /// The kinematic rigid body being driven.
    pub rigid_body_id: KinematicRigidBodyID,
    /// The constant acceleration trajectory imposed on the body.
    pub trajectory: ConstantAccelerationTrajectory,
}

define_setup_type! {
    /// A trajectory with constant acceleration.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ConstantAccelerationTrajectory {
        /// When (in simulation time) the body should be at the initial position.
        pub initial_time: fph,
        /// The position of the body at the initial time.
        pub initial_position: Position,
        /// The velocity of the body at the initial time.
        pub initial_velocity: Velocity,
        /// The constant acceleration of the body.
        pub acceleration: Acceleration,
    }
}

impl From<u64> for ConstantAccelerationTrajectoryDriverID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl ConstantAccelerationTrajectoryDriver {
    /// Resets the appropriate properties of the driven rigid body in
    /// preparation for applying driven properties.
    pub fn reset(&self, rigid_body_manager: &mut RigidBodyManager) {
        let Some(body) = rigid_body_manager.get_kinematic_rigid_body_mut(self.rigid_body_id) else {
            return;
        };
        body.set_position(Position::origin());
        body.set_velocity(Velocity::zeros());
    }

    /// Applies the driven properties for the given time to the appropriate
    /// rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, time: fph) {
        let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(self.rigid_body_id)
        else {
            return;
        };

        let (trajectory_position, trajectory_velocity) =
            self.trajectory.compute_position_and_velocity(time);

        rigid_body.set_position(rigid_body.position() + trajectory_position.coords);
        rigid_body.set_velocity(rigid_body.velocity() + trajectory_velocity);
    }
}

#[roc]
impl ConstantAccelerationTrajectory {
    /// Creates a new constant acceleration trajectory with the given properties.
    #[roc(body = r#"
    {
        initial_time,
        initial_position,
        initial_velocity,
        acceleration,
    }
    "#)]
    pub fn new(
        initial_time: fph,
        initial_position: Position,
        initial_velocity: Velocity,
        acceleration: Acceleration,
    ) -> Self {
        Self {
            initial_time,
            initial_position,
            initial_velocity,
            acceleration,
        }
    }

    /// Creates a new constant velocity trajectory (no acceleration) with the
    /// given properties.
    #[roc(body = r#"
    new(
        initial_time,
        initial_position,
        velocity,
        Vector3.zero,
    )
    "#)]
    pub fn with_constant_velocity(
        initial_time: fph,
        initial_position: Position,
        velocity: Velocity,
    ) -> Self {
        Self::new(
            initial_time,
            initial_position,
            velocity,
            Acceleration::zeros(),
        )
    }

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
            ConstantAccelerationTrajectory::with_constant_velocity(0.0, position, velocity);
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
        let trajectory = ConstantAccelerationTrajectory::new(
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
            let trajectory = ConstantAccelerationTrajectory::new(
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
