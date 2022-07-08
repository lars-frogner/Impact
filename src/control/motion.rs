use super::MotionController;
use crate::num::Float;
use approx::abs_diff_eq;
use nalgebra::{vector, Rotation3, Translation3, Vector3};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoMotionController;

#[derive(Clone, Debug)]
pub struct SemiDirectionalMotionController<F: Float> {
    semi_directional_motion: SemiDirectionalMotion<F>,
    time_of_last_motion: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionState {
    Still,
    Moving,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionDirection {
    Forwards,
    Backwards,
    Right,
    Left,
    Up,
    Down,
}

#[derive(Clone, Debug, Default)]
struct SemiDirectionalMotion<F: Float> {
    orientation: Rotation3<F>,
    movement_speed: F,
    state: SemiDirectionalMotionState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SemiDirectionalMotionState {
    forwards: MotionState,
    backwards: MotionState,
    right: MotionState,
    left: MotionState,
    up: MotionState,
    down: MotionState,
}

#[derive(Clone, Debug)]
enum Motion<F> {
    Stationary,
    ConstantVelocity(Vector3<F>),
}

impl<F> MotionController<F> for NoMotionController {
    fn next_translation(&mut self) -> Option<Translation3<F>> {
        None
    }

    fn update_motion(&mut self, _direction: MotionDirection, _state: MotionState) {}

    fn set_orientation(&mut self, _orientation: Rotation3<F>) {}

    fn rotate_orientation(&mut self, _rotation: &Rotation3<F>) {}

    fn set_movement_speed(&mut self, _movement_speed: F) {}

    fn stop(&mut self) {}
}

impl<F: Float> SemiDirectionalMotionController<F> {
    pub fn new(orientation: Rotation3<F>, movement_speed: F) -> Self {
        Self {
            semi_directional_motion: SemiDirectionalMotion::new(orientation, movement_speed),
            time_of_last_motion: Instant::now(),
        }
    }
}

impl<F: Float> MotionController<F> for SemiDirectionalMotionController<F> {
    fn next_translation(&mut self) -> Option<Translation3<F>> {
        match self.semi_directional_motion.compute_motion() {
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

    fn update_motion(&mut self, direction: MotionDirection, state: MotionState) {
        self.semi_directional_motion.state.update(direction, state);
    }

    fn set_orientation(&mut self, orientation: Rotation3<F>) {
        self.semi_directional_motion.orientation = orientation;
    }

    fn rotate_orientation(&mut self, rotation: &Rotation3<F>) {
        self.semi_directional_motion.orientation =
            rotation * self.semi_directional_motion.orientation;
    }

    fn set_movement_speed(&mut self, movement_speed: F) {
        self.semi_directional_motion.movement_speed = movement_speed;
    }

    fn stop(&mut self) {
        self.semi_directional_motion.state.stop();
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

impl<F: Float> SemiDirectionalMotion<F> {
    fn new(orientation: Rotation3<F>, movement_speed: F) -> Self {
        Self {
            orientation,
            movement_speed,
            state: SemiDirectionalMotionState::new(),
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
        if self.forwards == self.backwards && self.right == self.left && self.up == self.down {
            MotionState::Still
        } else {
            MotionState::Moving
        }
    }

    fn update(&mut self, direction: MotionDirection, state: MotionState) {
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
