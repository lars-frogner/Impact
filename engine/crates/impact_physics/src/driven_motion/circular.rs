//! Circular motion.

use crate::{
    driven_motion::MotionDriverRegistry,
    quantities::{OrientationC, Position, PositionC, Velocity, VelocityC},
    rigid_body::{KinematicRigidBodyID, RigidBodyManager},
};
use approx::abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_id::{EntityID, define_entity_id_newtype};
use impact_math::consts::f32::TWO_PI;
use roc_integration::roc;

/// Manages all [`CircularTrajectoryDriver`]s.
pub type CircularTrajectoryRegistry =
    MotionDriverRegistry<CircularTrajectoryDriverID, CircularTrajectoryDriver>;

define_entity_id_newtype! {
    /// Identifier for a [`CircularTrajectoryDriver`].
    [pub] CircularTrajectoryDriverID
}

define_component_type! {
    /// Marks that an entity has a circular trajectory driver identified by a
    /// [`CircularTrajectoryDriverID`].
    ///
    /// Use [`CircularTrajectoryDriverID::from_entity_id`] to obtain the driver
    /// ID from the entity ID.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct HasCircularTrajectoryDriver;
}

/// Driver for imposing a circular trajectory with constant speed on a kinematic
/// rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct CircularTrajectoryDriver {
    /// The entity being driven.
    pub entity_id: EntityID,
    /// The circular trajectory imposed on the body.
    pub trajectory: CircularTrajectory,
}

define_setup_type! {
    /// A circular trajectory with constant speed.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct CircularTrajectory {
        /// When (in simulation time) the body should be at the initial position
        /// on the circle.
        pub initial_time: f32,
        /// The orientation of the orbit. The first axis of the circle's reference
        /// frame will coincide with the direction from the center to the position
        /// of the body at the initial time, the second with the direction of the
        /// velocity at the initial time and the third with the normal of the
        /// circle's plane.
        pub orientation: OrientationC,
        /// The position of the center of the circle.
        pub center_position: PositionC,
        /// The radius of the circle.
        pub radius: f32,
        /// The duration of one revolution.
        pub period: f32,
    }
}

impl CircularTrajectoryDriver {
    /// Resets the appropriate properties of the driven rigid body in
    /// preparation for applying driven properties.
    pub fn reset(&self, rigid_body_manager: &mut RigidBodyManager) {
        let rigid_body_id = KinematicRigidBodyID::from_entity_id(self.entity_id);
        let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(rigid_body_id)
        else {
            return;
        };
        rigid_body.set_position(PositionC::origin());
        rigid_body.set_velocity(VelocityC::zeros());
    }

    /// Applies the driven properties for the given time to the appropriate
    /// rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, time: f32) {
        let rigid_body_id = KinematicRigidBodyID::from_entity_id(self.entity_id);
        let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(rigid_body_id)
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
impl CircularTrajectory {
    /// Creates a new circular trajectory with the given properties.
    #[roc(body = r#"
    {
        initial_time,
        orientation,
        center_position,
        radius,
        period,
    }
    "#)]
    pub fn new(
        initial_time: f32,
        orientation: OrientationC,
        center_position: PositionC,
        radius: f32,
        period: f32,
    ) -> Self {
        Self {
            initial_time,
            orientation,
            center_position,
            radius,
            period,
        }
    }

    /// Computes the position and velocity for the trajectory at the given time.
    ///
    /// # Panics
    /// - If the radius does not exceed zero.
    /// - If the period is zero.
    pub fn compute_position_and_velocity(&self, time: f32) -> (PositionC, VelocityC) {
        assert!(
            self.radius > 0.0,
            "Radius of circular trajectory does not exceed zero"
        );
        assert!(
            abs_diff_ne!(self.period, 0.0),
            "Period of circular trajectory is zero"
        );

        let orientation = self.orientation.aligned();

        let angular_speed = Self::compute_angular_speed(self.period);

        let angle = Self::compute_angle(self.initial_time, angular_speed, time);
        let (sin_angle, cos_angle) = angle.sin_cos();

        let circular_displacement =
            Self::compute_circular_displacement(self.radius, cos_angle, sin_angle);

        let world_space_circular_displacement = orientation.rotate_point(&circular_displacement);

        let world_space_circular_position =
            self.center_position.aligned() + world_space_circular_displacement.as_vector();

        let tangential_speed = self.radius * angular_speed;

        let circular_velocity =
            Self::compute_circular_velocity(cos_angle, sin_angle, tangential_speed);

        let world_space_circular_velocity = orientation.rotate_vector(&circular_velocity);

        (
            world_space_circular_position.compact(),
            world_space_circular_velocity.compact(),
        )
    }

    fn compute_angular_speed(period: f32) -> f32 {
        TWO_PI / period
    }

    fn compute_angle(initial_time: f32, angular_speed: f32, time: f32) -> f32 {
        angular_speed * (time - initial_time) % TWO_PI
    }

    fn compute_circular_displacement(radius: f32, cos_angle: f32, sin_angle: f32) -> Position {
        Position::new(radius * cos_angle, radius * sin_angle, 0.0)
    }

    fn compute_circular_velocity(
        cos_angle: f32,
        sin_angle: f32,
        tangential_speed: f32,
    ) -> Velocity {
        Velocity::new(
            -tangential_speed * sin_angle,
            tangential_speed * cos_angle,
            0.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantities::{DirectionC, Orientation, OrientationC};
    use approx::abs_diff_eq;
    use impact_math::{
        consts::f32::{FRAC_PI_2, PI},
        vector::UnitVector3C,
    };
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
    fn should_panic_if_radius_is_zero() {
        let trajectory =
            CircularTrajectory::new(0.0, OrientationC::identity(), PositionC::origin(), 0.0, 1.0);
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_radius_is_negative() {
        let trajectory = CircularTrajectory::new(
            0.0,
            OrientationC::identity(),
            PositionC::origin(),
            -0.1,
            1.0,
        );
        trajectory.compute_position_and_velocity(1.0);
    }

    #[test]
    #[should_panic]
    fn should_panic_if_period_is_zero() {
        let trajectory =
            CircularTrajectory::new(0.0, OrientationC::identity(), PositionC::origin(), 1.0, 0.0);
        trajectory.compute_position_and_velocity(1.0);
    }

    proptest! {
        #[test]
        fn should_get_antiparallel_velocities_at_half_period_offset(
            initial_time in -1e2..1e2_f32,
            orientation in orientation_strategy(),
            center_position in position_strategy(1e2),
            radius in 1e-2..1e2_f32,
            period in 1e-2..1e2_f32,
            time in -1e2..1e2_f32,
        ) {
            let trajectory = CircularTrajectory::new(
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
                .normalized();
            let second_velocity_direction = trajectory
                .compute_position_and_velocity(half_period_offset_time)
                .1
                .normalized();

            prop_assert!(abs_diff_eq!(
                first_velocity_direction.dot(&second_velocity_direction),
                -1.0,
                epsilon = 1e-5
            ));
        }
    }

    proptest! {
        #[test]
        fn should_get_circular_position_and_velocity(
            initial_time in -1e2..1e2_f32,
            orientation in orientation_strategy(),
            center_position in position_strategy(1e2),
            radius in 1e-2..1e2_f32,
            period in 1e-2..1e2_f32,
            time in -1e2..1e2_f32,
        ) {
            let trajectory = CircularTrajectory::new(
                initial_time,
                orientation,
                center_position,
                radius,
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
