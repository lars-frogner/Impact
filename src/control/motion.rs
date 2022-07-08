//! Motion controller implementations.

use super::MotionController;
use crate::num::Float;
use approx::abs_diff_eq;
use nalgebra::{vector, Rotation3, Translation3, Vector3};
use std::time::Instant;

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
    time_of_last_motion: Instant,
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

/// Possible types of motion.
#[derive(Clone, Debug)]
enum Motion<F> {
    Stationary,
    ConstantVelocity(Vector3<F>),
}

impl<F> MotionController<F> for NoMotionController {
    fn next_translation(&mut self) -> Option<Translation3<F>> {
        None
    }

    fn update_motion(&mut self, _state: MotionState, _direction: MotionDirection) {}

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
            time_of_last_motion: Instant::now(),
        }
    }

    fn compute_motion(&self) -> Motion<F> {
        if self.state.motion_state() == MotionState::Still
            || abs_diff_eq!(self.movement_speed, F::zero())
        {
            Motion::Stationary
        } else {
            let velocity_x = if self.state.right == self.state.left {
                F::zero()
            } else if self.state.right.is_moving() {
                self.movement_speed
            } else {
                -self.movement_speed
            };
            let velocity_y = if self.state.up == self.state.down {
                F::zero()
            } else if self.state.up.is_moving() {
                self.movement_speed
            } else {
                -self.movement_speed
            };
            let velocity_z = if self.state.forwards == self.state.backwards {
                F::zero()
            } else if self.state.forwards.is_moving() {
                self.movement_speed
            } else {
                -self.movement_speed
            };
            Motion::ConstantVelocity(self.orientation * vector![velocity_x, velocity_y, velocity_z])
        }
    }
}

impl<F: Float> MotionController<F> for SemiDirectionalMotionController<F> {
    fn next_translation(&mut self) -> Option<Translation3<F>> {
        match self.compute_motion() {
            Motion::Stationary => None,
            Motion::ConstantVelocity(velocity) => {
                let current_time = Instant::now();
                let elapsed_time = current_time - self.time_of_last_motion;
                let translation = velocity * F::from_f64(elapsed_time.as_secs_f64()).unwrap();
                self.time_of_last_motion = current_time;
                Some(translation.into())
            }
        }
    }

    fn update_motion(&mut self, state: MotionState, direction: MotionDirection) {
        self.state.update(state, direction);
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

    fn update(&mut self, state: MotionState, direction: MotionDirection) {
        match direction {
            MotionDirection::Forwards => self.forwards = state,
            MotionDirection::Backwards => self.backwards = state,
            MotionDirection::Right => self.right = state,
            MotionDirection::Left => self.left = state,
            MotionDirection::Up => self.up = state,
            MotionDirection::Down => self.down = state,
        };
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
