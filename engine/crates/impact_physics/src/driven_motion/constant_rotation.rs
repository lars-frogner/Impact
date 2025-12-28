//! Constant rotation.

use crate::{
    driven_motion::MotionDriverRegistry,
    quantities::{AngularVelocityP, OrientationP},
    rigid_body::advance_orientation,
    rigid_body::{KinematicRigidBodyID, RigidBodyManager},
};
use bytemuck::{Pod, Zeroable};
use roc_integration::roc;

/// Manages all [`ConstantRotationDriver`]s.
pub type ConstantRotationRegistry =
    MotionDriverRegistry<ConstantRotationDriverID, ConstantRotationDriver>;

define_component_type! {
    /// Identifier for a [`ConstantRotationDriver`].
    #[roc(parents = "Comp")]
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Zeroable, Pod)]
    pub struct ConstantRotationDriverID(u64);
}

/// Driver for imposing constant rotation on a kinematic rigid body.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ConstantRotationDriver {
    /// The kinematic rigid body being driven.
    pub rigid_body_id: KinematicRigidBodyID,
    /// The constant rotation imposed on the body.
    pub rotation: ConstantRotation,
    padding: f32,
}

define_setup_type! {
    /// Constant rotation with a fixed angular velocity.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ConstantRotation {
        /// When (in simulation time) the body should have the initial
        /// orientation.
        pub initial_time: f32,
        /// The orientation of the body at the initial time.
        pub initial_orientation: OrientationP,
        /// The angular velocity of the body.
        pub angular_velocity: AngularVelocityP,
    }
}

impl From<u64> for ConstantRotationDriverID {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl ConstantRotationDriver {
    pub fn new(rigid_body_id: KinematicRigidBodyID, rotation: ConstantRotation) -> Self {
        Self {
            rigid_body_id,
            rotation,
            padding: 0.0,
        }
    }

    /// Applies the driven properties for the given time to the appropriate
    /// rigid body.
    pub fn apply(&self, rigid_body_manager: &mut RigidBodyManager, time: f32) {
        let Some(rigid_body) = rigid_body_manager.get_kinematic_rigid_body_mut(self.rigid_body_id)
        else {
            return;
        };

        let trajectory_orientation = self.rotation.compute_orientation(time);

        rigid_body.set_orientation(trajectory_orientation);
        rigid_body.set_angular_velocity(self.rotation.angular_velocity);
    }
}

#[roc]
impl ConstantRotation {
    /// Creates a new constant rotation defined by the given initial time and
    /// orientation and angular velocity.
    #[roc(body = r#"
    {
        initial_time,
        initial_orientation,
        angular_velocity,
    }
    "#)]
    pub fn new(
        initial_time: f32,
        initial_orientation: OrientationP,
        angular_velocity: AngularVelocityP,
    ) -> Self {
        Self {
            initial_time,
            initial_orientation,
            angular_velocity,
        }
    }

    /// Computes the orientation at the given time.
    pub fn compute_orientation(&self, time: f32) -> OrientationP {
        let initial_orientation = self.initial_orientation.unpack();
        let angular_velocity = self.angular_velocity.unpack();

        let time_offset = time - self.initial_time;

        let orientation = advance_orientation(&initial_orientation, &angular_velocity, time_offset);

        orientation.pack()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantities::{AngularVelocityP, DirectionP, Orientation};
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
    use impact_math::{
        Float,
        angle::Radians,
        vector::{UnitVector3P, Vector3P},
    };
    use proptest::prelude::*;

    prop_compose! {
        fn direction_strategy()(
            phi in 0.0..f32::TWO_PI,
            theta in 0.0..f32::PI,
        ) -> DirectionP {
            DirectionP::normalized_from(Vector3P::new(
                f32::cos(phi) * f32::sin(theta),
                f32::sin(phi) * f32::sin(theta),
                f32::cos(theta)
            ))
        }
    }

    prop_compose! {
        fn orientation_strategy()(
            rotation_roll in 0.0..f32::TWO_PI,
            rotation_pitch in -f32::FRAC_PI_2..f32::FRAC_PI_2,
            rotation_yaw in 0.0..f32::TWO_PI,
        ) -> OrientationP {
            Orientation::from_euler_angles(rotation_roll, rotation_pitch, rotation_yaw).pack()
        }
    }

    prop_compose! {
        fn angular_velocity_strategy(max_angular_speed: f32)(
            angular_speed in -max_angular_speed..max_angular_speed,
            axis in direction_strategy(),
        ) -> AngularVelocityP {
            AngularVelocityP::new(axis, Radians(angular_speed))
        }
    }

    #[test]
    fn should_get_initial_orientation_for_zero_angular_velocity() {
        let orientation = OrientationP::identity();
        let angular_velocity = AngularVelocityP::zero();
        let rotation = ConstantRotation::new(0.0, orientation, angular_velocity);
        let rotated_orientation = rotation.compute_orientation(1.0);
        assert_abs_diff_eq!(rotated_orientation, orientation, epsilon = 1e-6);
    }

    #[test]
    fn should_get_different_orientation_for_nonzero_angular_velocity() {
        let orientation = OrientationP::identity();
        let angular_velocity = AngularVelocityP::new(UnitVector3P::unit_y(), Radians(1.0));
        let rotation = ConstantRotation::new(0.0, orientation, angular_velocity);
        let rotated_orientation = rotation.compute_orientation(1.0);
        assert_abs_diff_ne!(rotated_orientation, orientation, epsilon = 1e-6);
    }

    proptest! {
        #[test]
        fn should_get_initial_orientation_at_initial_time(
            time in -1e2..1e2_f32,
            orientation in orientation_strategy(),
            angular_velocity in angular_velocity_strategy(1e2),
        ) {
            let rotation = ConstantRotation::new(
                time,
                orientation,
                angular_velocity
            );
            let rotated_orientation = rotation.compute_orientation(time);
            prop_assert!(abs_diff_eq!(rotated_orientation, orientation, epsilon = 1e-6));
        }
    }
}
