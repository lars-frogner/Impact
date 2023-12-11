//! Rotation with a constant angular velocity.

use crate::physics::{self, fph, AngularVelocity, Orientation};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that rotate
/// with a constant angular velocity over time.
///
/// For this component to have an effect, the entity also needs a
/// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ConstantRotationComp {
    /// When (in simulation time) the entity should have the initial
    /// orientation.
    pub initial_time: fph,
    /// The orientation of the entity at the initial time.
    pub initial_orientation: Orientation,
    /// The angular velocity of the entity.
    pub angular_velocity: AngularVelocity,
}

impl ConstantRotationComp {
    /// Creates a new component for constant rotation defined by the given
    /// initial time and orientation and angular velocity.
    pub fn new(
        initial_time: fph,
        initial_orientation: Orientation,
        angular_velocity: AngularVelocity,
    ) -> Self {
        Self {
            initial_time,
            initial_orientation,
            angular_velocity,
        }
    }

    /// Computes the orientation at the given time.
    pub fn compute_orientation(&self, time: fph) -> Orientation {
        let time_offset = time - self.initial_time;
        physics::advance_orientation(
            &self.initial_orientation,
            &self.angular_velocity,
            time_offset,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{geometry::Radians, num::Float, physics::Direction};
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
    use nalgebra::{vector, Vector3};
    use proptest::prelude::*;

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

    prop_compose! {
        fn angular_velocity_strategy(max_angular_speed: fph)(
            angular_speed in -max_angular_speed..max_angular_speed,
            axis in direction_strategy(),
        ) -> AngularVelocity {
            AngularVelocity::new(axis, Radians(angular_speed))
        }
    }

    #[test]
    fn should_get_initial_orientation_for_zero_angular_velocity() {
        let orientation = Orientation::identity();
        let angular_velocity = AngularVelocity::zero();
        let rotation = ConstantRotationComp::new(0.0, orientation, angular_velocity);
        let rotated_orientation = rotation.compute_orientation(1.0);
        assert_abs_diff_eq!(rotated_orientation, orientation);
    }

    #[test]
    fn should_get_different_orientation_for_nonzero_angular_velocity() {
        let orientation = Orientation::identity();
        let angular_velocity = AngularVelocity::new(Vector3::y_axis(), Radians(1.0));
        let rotation = ConstantRotationComp::new(0.0, orientation, angular_velocity);
        let rotated_orientation = rotation.compute_orientation(1.0);
        assert_abs_diff_ne!(rotated_orientation, orientation);
    }

    proptest! {
        #[test]
        fn should_get_initial_orientation_at_initial_time(
            time in -1e2..1e2,
            orientation in orientation_strategy(),
            angular_velocity in angular_velocity_strategy(1e2),
        ) {
            let rotation = ConstantRotationComp::new(
                time,
                orientation,
                angular_velocity
            );
            let rotated_orientation = rotation.compute_orientation(time);
            prop_assert!(abs_diff_eq!(rotated_orientation, orientation));
        }
    }
}
