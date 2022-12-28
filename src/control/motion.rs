//! Motion controller implementations.

use super::{ControlledMotion, MotionController};
use crate::num::Float;
use approx::{abs_diff_eq, assert_abs_diff_ne, AbsDiffEq};
use nalgebra::{vector, Rotation3};

/// Motion controller that allows no control over motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoMotionController;

/// Motion controller allowing for motion at constant
/// speed along the axes of an entity's local coordinate
/// system (`W-A-S-D` type motion).
#[derive(Clone, Debug)]
pub struct SemiDirectionalMotionController<F: Float> {
    orientation: Rotation3<F>,
    movement_speed: F,
    state: SemiDirectionalMotionState,
    motion: ControlledMotion<F>,
}

/// Whether there is motion in a certain direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionState {
    Still,
    Moving,
}

/// Possible directions of motion in the local coordinate
/// system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionDirection {
    Forwards,
    Backwards,
    Right,
    Left,
    Up,
    Down,
}

/// Whether there is motion in each local direction.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SemiDirectionalMotionState {
    forwards: MotionState,
    backwards: MotionState,
    right: MotionState,
    left: MotionState,
    up: MotionState,
    down: MotionState,
}

impl<F: Float> MotionController<F> for NoMotionController {
    fn current_motion(&self) -> &ControlledMotion<F> {
        &ControlledMotion::Stationary
    }

    fn update_motion(&mut self, _state: MotionState, _direction: MotionDirection) -> bool {
        false
    }

    fn set_orientation(&mut self, _orientation: Rotation3<F>) {}

    fn rotate_orientation(&mut self, _rotation: &Rotation3<F>) {}

    fn set_movement_speed(&mut self, _movement_speed: F) {}

    fn stop(&mut self) {}
}

impl<F: Float> SemiDirectionalMotionController<F> {
    /// Creates a new motion controller for an entity
    /// whose local coordinate system has the given
    /// orientation and that can move with the given speed.
    pub fn new(orientation: Rotation3<F>, movement_speed: F) -> Self {
        Self {
            orientation,
            movement_speed,
            state: SemiDirectionalMotionState::new(),
            motion: ControlledMotion::Stationary,
        }
    }

    fn compute_motion(&self) -> ControlledMotion<F> {
        if self.state.motion_state() == MotionState::Still
            || abs_diff_eq!(self.movement_speed, F::zero())
        {
            ControlledMotion::Stationary
        } else {
            // For scaling the magnitude to `self.movement_speed`
            let mut n_nonzero_components = F::zero();

            let velocity_x = if self.state.right == self.state.left {
                F::zero()
            } else {
                n_nonzero_components += F::one();
                if self.state.right.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            let velocity_y = if self.state.up == self.state.down {
                F::zero()
            } else {
                n_nonzero_components += F::one();
                if self.state.up.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            let velocity_z = if self.state.forwards == self.state.backwards {
                F::zero()
            } else {
                n_nonzero_components += F::one();
                if self.state.forwards.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            // We should have motion in this branch
            assert_abs_diff_ne!(n_nonzero_components, F::zero());

            let magnitude_scale = F::one() / F::sqrt(n_nonzero_components);

            ControlledMotion::ConstantVelocity(
                self.orientation * vector![velocity_x, velocity_y, velocity_z] * magnitude_scale,
            )
        }
    }
}

impl<F: Float> MotionController<F> for SemiDirectionalMotionController<F> {
    fn current_motion(&self) -> &ControlledMotion<F> {
        &self.motion
    }

    fn update_motion(&mut self, state: MotionState, direction: MotionDirection) -> bool {
        let changed = self.state.update(state, direction);
        if changed {
            self.motion = self.compute_motion();
        }
        changed
    }

    fn set_orientation(&mut self, orientation: Rotation3<F>) {
        self.orientation = orientation;
    }

    fn rotate_orientation(&mut self, rotation: &Rotation3<F>) {
        self.orientation = rotation * self.orientation;
    }

    fn set_movement_speed(&mut self, movement_speed: F) {
        self.movement_speed = movement_speed;
    }

    fn stop(&mut self) {
        self.state.stop();
    }
}

impl MotionState {
    pub fn is_still(&self) -> bool {
        *self == Self::Still
    }

    pub fn is_moving(&self) -> bool {
        *self == Self::Moving
    }

    pub fn update(&mut self, state: Self) -> bool {
        let changed = self != &state;
        *self = state;
        changed
    }
}

impl SemiDirectionalMotionState {
    fn new() -> Self {
        Self {
            forwards: MotionState::Still,
            backwards: MotionState::Still,
            right: MotionState::Still,
            left: MotionState::Still,
            up: MotionState::Still,
            down: MotionState::Still,
        }
    }

    fn motion_state(&self) -> MotionState {
        // This takes into account that motion in oppsite
        // directions will be cancelled out
        if self.forwards == self.backwards && self.right == self.left && self.up == self.down {
            MotionState::Still
        } else {
            MotionState::Moving
        }
    }

    fn update(&mut self, state: MotionState, direction: MotionDirection) -> bool {
        match direction {
            MotionDirection::Forwards => self.forwards.update(state),
            MotionDirection::Backwards => self.backwards.update(state),
            MotionDirection::Right => self.right.update(state),
            MotionDirection::Left => self.left.update(state),
            MotionDirection::Up => self.up.update(state),
            MotionDirection::Down => self.down.update(state),
        }
    }

    fn stop(&mut self) {
        self.forwards = MotionState::Still;
        self.backwards = MotionState::Still;
        self.right = MotionState::Still;
        self.left = MotionState::Still;
        self.up = MotionState::Still;
        self.down = MotionState::Still;
    }
}

impl Default for SemiDirectionalMotionState {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Float> AbsDiffEq for ControlledMotion<F>
where
    F: Copy + AbsDiffEq,
    F::Epsilon: Copy,
{
    type Epsilon = F::Epsilon;

    fn default_epsilon() -> F::Epsilon {
        F::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: F::Epsilon) -> bool {
        match (self, other) {
            (Self::Stationary, Self::Stationary) => true,
            (Self::ConstantVelocity(velocity_self), Self::ConstantVelocity(velocity_other)) => {
                velocity_self.abs_diff_eq(velocity_other, epsilon)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::Vector3;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, SQRT_2};
    use MotionDirection::*;
    use MotionState::*;

    #[test]
    fn updating_semi_directional_motion_works() {
        let speed = 1.3;
        let mut controller = SemiDirectionalMotionController::new(Rotation3::identity(), speed);
        assert_eq!(
            controller.compute_motion(),
            ControlledMotion::Stationary,
            "Not stationary directly after initalization"
        );

        assert!(controller.update_motion(Moving, Forwards));
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, 0.0, speed]),
        );

        assert!(!controller.update_motion(Moving, Forwards));

        assert!(controller.update_motion(Moving, Backwards));
        assert_eq!(
            controller.compute_motion(),
            ControlledMotion::Stationary,
            "Motion does not cancel"
        );

        assert!(controller.update_motion(Moving, Left));
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![-speed, 0.0, 0.0])
        );

        controller.stop();
        assert_eq!(
            controller.compute_motion(),
            ControlledMotion::Stationary,
            "Stopping command not working"
        );

        // Motion along multiple axes should be combined
        assert!(controller.update_motion(Moving, Up));
        assert!(controller.update_motion(Moving, Backwards));
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, speed, -speed] / SQRT_2), // Magnitude should be `speed`
            epsilon = 1e-9
        );

        assert!(controller.update_motion(Still, Up));
        assert!(controller.update_motion(Still, Backwards));
        assert_eq!(
            controller.compute_motion(),
            ControlledMotion::Stationary,
            "Undoing updates does not stop motion"
        );
    }

    #[test]
    fn orientation_of_semi_directional_motion_works() {
        let speed = 2.2;
        let mut controller = SemiDirectionalMotionController::new(
            Rotation3::from_axis_angle(&Vector3::y_axis(), PI),
            speed,
        );
        assert_eq!(
            controller.compute_motion(),
            ControlledMotion::Stationary,
            "Not stationary directly after initalization"
        );

        controller.update_motion(Moving, Forwards);
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, 0.0, -speed]), // Should move backwards due to rotation
            epsilon = 1e-9
        );

        controller.rotate_orientation(&Rotation3::from_axis_angle(&Vector3::x_axis(), FRAC_PI_2));
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, speed, 0.0]), // The additional rotation points us upwards
            epsilon = 1e-9
        );

        controller.set_orientation(Rotation3::identity());
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, 0.0, speed]),
            epsilon = 1e-9
        );

        controller.set_orientation(Rotation3::from_axis_angle(&Vector3::y_axis(), -FRAC_PI_4));
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![-speed, 0.0, speed] / SQRT_2), // Magnitude should be `speed`
            epsilon = 1e-9
        );
    }

    #[test]
    fn setting_semi_directional_motion_speed_works() {
        let speed = 4.2;
        let mut controller = SemiDirectionalMotionController::new(Rotation3::identity(), speed);

        controller.update_motion(Moving, Down);
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, -speed, 0.0]),
        );

        let speed = 8.1;
        controller.set_movement_speed(speed);
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, -speed, 0.0]),
        );

        let speed = -0.1;
        controller.set_movement_speed(speed);
        assert_abs_diff_eq!(
            controller.compute_motion(),
            ControlledMotion::ConstantVelocity(vector![0.0, -speed, 0.0]),
        );

        controller.set_movement_speed(0.0);
        assert_eq!(controller.compute_motion(), ControlledMotion::Stationary,);
    }
}
