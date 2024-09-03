//! Motion controller implementations.

pub mod components;
pub mod systems;

use super::{MotionChanged, MotionController};
use crate::{
    num::Float,
    physics::{
        fph,
        motion::{Orientation, Velocity},
    },
};
use approx::{abs_diff_eq, assert_abs_diff_ne};
use nalgebra::vector;

/// Motion controller allowing for motion at constant
/// speed along the axes of an entity's local coordinate
/// system (`W-A-S-D` type motion).
#[derive(Clone, Debug)]
pub struct SemiDirectionalMotionController {
    movement_speed: fph,
    vertical_control: bool,
    state: SemiDirectionalMotionState,
    local_velocity: Velocity,
}

/// Whether there is motion in a certain direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionState {
    Still,
    Moving,
}

/// Possible directions of motion in the local coordinate
/// system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl SemiDirectionalMotionController {
    /// Creates a new motion controller for an entity
    /// that can move with the given `movement_speed`.
    /// `vertical_control` specifies whether the controller
    /// can move the entity in the vertical direction.
    pub fn new(movement_speed: fph, vertical_control: bool) -> Self {
        Self {
            movement_speed,
            vertical_control,
            state: SemiDirectionalMotionState::new(),
            local_velocity: Velocity::zeros(),
        }
    }

    /// Computes the velocity of the controlled entity in its local
    /// coordinate system.
    fn compute_local_velocity(&self) -> Velocity {
        if self.state.motion_state() == MotionState::Still
            || abs_diff_eq!(self.movement_speed, fph::ZERO)
        {
            Velocity::zeros()
        } else {
            // For scaling the magnitude to unity
            let mut n_nonzero_components = fph::ZERO;

            let velocity_x = if self.state.right == self.state.left {
                fph::ZERO
            } else {
                n_nonzero_components += fph::ONE;
                if self.state.right.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            let velocity_y = if self.state.up == self.state.down {
                fph::ZERO
            } else {
                n_nonzero_components += fph::ONE;
                if self.state.up.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            let velocity_z = if self.state.forwards == self.state.backwards {
                fph::ZERO
            } else {
                n_nonzero_components += fph::ONE;
                if self.state.forwards.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            // We should have motion in this branch
            assert_abs_diff_ne!(n_nonzero_components, fph::ZERO);

            let magnitude_scale = fph::ONE / fph::sqrt(n_nonzero_components);

            vector![velocity_x, velocity_y, velocity_z] * magnitude_scale
        }
    }
}

impl MotionController for SemiDirectionalMotionController {
    fn movement_speed(&self) -> fph {
        self.movement_speed
    }

    fn compute_control_velocity(&self, orientation: &Orientation) -> Velocity {
        let mut control_velocity = orientation.transform_vector(&self.local_velocity);
        if !self.vertical_control {
            control_velocity.y = 0.0;
        }
        control_velocity
    }

    fn update_motion(&mut self, state: MotionState, direction: MotionDirection) -> MotionChanged {
        let result = self.state.update(state, direction);
        if result.motion_changed() {
            self.local_velocity = self.compute_local_velocity();
        }
        result
    }

    fn set_movement_speed(&mut self, movement_speed: fph) -> MotionChanged {
        if movement_speed != self.movement_speed {
            self.movement_speed = movement_speed;
            self.local_velocity = self.compute_local_velocity();
            MotionChanged::Yes
        } else {
            MotionChanged::No
        }
    }

    fn stop(&mut self) -> MotionChanged {
        if self.state.motion_state().is_moving() {
            self.state.stop();
            self.local_velocity = self.compute_local_velocity();
            MotionChanged::Yes
        } else {
            MotionChanged::No
        }
    }
}

impl MotionState {
    pub fn is_still(&self) -> bool {
        *self == Self::Still
    }

    pub fn is_moving(&self) -> bool {
        *self == Self::Moving
    }

    pub fn update(&mut self, state: Self) -> MotionChanged {
        if self != &state {
            *self = state;
            MotionChanged::Yes
        } else {
            MotionChanged::No
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
        // This takes into account that motion in oppsite
        // directions will be cancelled out
        if self.forwards == self.backwards && self.right == self.left && self.up == self.down {
            MotionState::Still
        } else {
            MotionState::Moving
        }
    }

    fn update(&mut self, state: MotionState, direction: MotionDirection) -> MotionChanged {
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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use std::f64::consts::SQRT_2;
    use MotionDirection::{Backwards, Down, Forwards, Left, Up};
    use MotionState::{Moving, Still};

    #[test]
    fn updating_semi_directional_motion_works() {
        let speed = 1.3;
        let mut controller = SemiDirectionalMotionController::new(speed, false);
        assert_eq!(
            controller.local_velocity,
            Velocity::zeros(),
            "Not stationary directly after initalization"
        );

        assert!(controller.update_motion(Moving, Forwards).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, vector![0.0, 0.0, speed],);

        assert!(!controller.update_motion(Moving, Forwards).motion_changed());

        assert!(controller.update_motion(Moving, Backwards).motion_changed());
        assert_eq!(
            controller.local_velocity,
            Velocity::zeros(),
            "Motion does not cancel"
        );

        assert!(controller.update_motion(Moving, Left).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, vector![-speed, 0.0, 0.0]);

        assert!(controller.stop().motion_changed());
        assert_eq!(
            controller.local_velocity,
            Velocity::zeros(),
            "Stopping command not working"
        );

        // Motion along multiple axes should be combined
        assert!(controller.update_motion(Moving, Up).motion_changed());
        assert!(controller.update_motion(Moving, Backwards).motion_changed());
        assert_abs_diff_eq!(
            controller.local_velocity,
            vector![0.0, speed, -speed] / SQRT_2, // Magnitude should be `speed`
            epsilon = 1e-9
        );

        assert!(controller.update_motion(Still, Up).motion_changed());
        assert!(controller.update_motion(Still, Backwards).motion_changed());
        assert_eq!(
            controller.local_velocity,
            Velocity::zeros(),
            "Undoing updates does not stop motion"
        );
    }

    #[test]
    fn setting_semi_directional_motion_speed_works() {
        let speed = 4.2;
        let mut controller = SemiDirectionalMotionController::new(speed, false);

        controller.update_motion(Moving, Down);
        assert_abs_diff_eq!(controller.local_velocity, vector![0.0, -speed, 0.0],);

        let speed = 8.1;
        assert!(controller.set_movement_speed(speed).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, vector![0.0, -speed, 0.0],);

        let speed = -0.1;
        assert!(controller.set_movement_speed(speed).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, vector![0.0, -speed, 0.0],);

        assert!(!controller.set_movement_speed(speed).motion_changed());

        assert!(controller.set_movement_speed(0.0).motion_changed());
        assert_eq!(controller.local_velocity, Velocity::zeros());
    }
}
