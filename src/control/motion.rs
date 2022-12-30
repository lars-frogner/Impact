//! Motion controller implementations.

use super::MotionController;
use crate::{
    control::Controllable,
    num::Float,
    physics::{fph, OrientationComp, Velocity, VelocityComp},
};
use approx::{abs_diff_eq, assert_abs_diff_ne};
use impact_ecs::{query, world::World as ECSWorld};
use nalgebra::vector;

/// Motion controller that allows no control over motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoMotionController;

/// Motion controller allowing for motion at constant
/// speed along the axes of an entity's local coordinate
/// system (`W-A-S-D` type motion).
#[derive(Clone, Debug)]
pub struct SemiDirectionalMotionController {
    movement_speed: fph,
    state: SemiDirectionalMotionState,
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

impl MotionController for NoMotionController {
    fn update_motion(
        &mut self,
        _ecs_world: &ECSWorld,
        _state: MotionState,
        _direction: MotionDirection,
    ) {
    }

    fn set_movement_speed(&mut self, _ecs_world: &ECSWorld, _movement_speed: fph) {}

    fn stop(&mut self, _ecs_world: &ECSWorld) {}
}

impl SemiDirectionalMotionController {
    /// Creates a new motion controller for an entity
    /// that can move with the given speed.
    pub fn new(movement_speed: fph) -> Self {
        Self {
            movement_speed,
            state: SemiDirectionalMotionState::new(),
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

    fn update_motion_no_ecs(&mut self, state: MotionState, direction: MotionDirection) -> bool {
        self.state.update(state, direction)
    }

    fn set_movement_speed_no_ecs(&mut self, movement_speed: fph) -> bool {
        let changed = movement_speed != self.movement_speed;
        self.movement_speed = movement_speed;
        changed
    }

    fn stop_no_ecs(&mut self) -> bool {
        let changed = self.state.motion_state().is_moving();
        self.state.stop();
        changed
    }

    fn update_controlled_entity_velocity(&self, ecs_world: &ECSWorld) {
        let local_velocity = self.compute_local_velocity();
        query!(
            ecs_world,
            |velocity: &mut VelocityComp, orientation: &OrientationComp| {
                let world_velocity = orientation.0.transform_vector(&local_velocity);
                velocity.0 = world_velocity;
            },
            [Controllable]
        );
    }
}

impl MotionController for SemiDirectionalMotionController {
    fn update_motion(
        &mut self,
        ecs_world: &ECSWorld,
        state: MotionState,
        direction: MotionDirection,
    ) {
        if self.update_motion_no_ecs(state, direction) {
            self.update_controlled_entity_velocity(ecs_world);
        }
    }

    fn set_movement_speed(&mut self, ecs_world: &ECSWorld, movement_speed: fph) {
        if self.set_movement_speed_no_ecs(movement_speed) {
            self.update_controlled_entity_velocity(ecs_world);
        }
    }

    fn stop(&mut self, ecs_world: &ECSWorld) {
        if self.stop_no_ecs() {
            self.update_controlled_entity_velocity(ecs_world);
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

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use std::f64::consts::SQRT_2;
    use MotionDirection::*;
    use MotionState::*;

    #[test]
    fn updating_semi_directional_motion_works() {
        let speed = 1.3;
        let mut controller = SemiDirectionalMotionController::new(speed);
        assert_eq!(
            controller.compute_local_velocity(),
            Velocity::zeros(),
            "Not stationary directly after initalization"
        );

        assert!(controller.update_motion_no_ecs(Moving, Forwards));
        assert_abs_diff_eq!(
            controller.compute_local_velocity(),
            vector![0.0, 0.0, speed],
        );

        assert!(!controller.update_motion_no_ecs(Moving, Forwards));

        assert!(controller.update_motion_no_ecs(Moving, Backwards));
        assert_eq!(
            controller.compute_local_velocity(),
            Velocity::zeros(),
            "Motion does not cancel"
        );

        assert!(controller.update_motion_no_ecs(Moving, Left));
        assert_abs_diff_eq!(
            controller.compute_local_velocity(),
            vector![-speed, 0.0, 0.0]
        );

        assert!(controller.stop_no_ecs());
        assert_eq!(
            controller.compute_local_velocity(),
            Velocity::zeros(),
            "Stopping command not working"
        );

        // Motion along multiple axes should be combined
        assert!(controller.update_motion_no_ecs(Moving, Up));
        assert!(controller.update_motion_no_ecs(Moving, Backwards));
        assert_abs_diff_eq!(
            controller.compute_local_velocity(),
            vector![0.0, speed, -speed] / SQRT_2, // Magnitude should be `speed`
            epsilon = 1e-9
        );

        assert!(controller.update_motion_no_ecs(Still, Up));
        assert!(controller.update_motion_no_ecs(Still, Backwards));
        assert_eq!(
            controller.compute_local_velocity(),
            Velocity::zeros(),
            "Undoing updates does not stop motion"
        );
    }

    #[test]
    fn setting_semi_directional_motion_speed_works() {
        let speed = 4.2;
        let mut controller = SemiDirectionalMotionController::new(speed);

        controller.update_motion_no_ecs(Moving, Down);
        assert_abs_diff_eq!(
            controller.compute_local_velocity(),
            vector![0.0, -speed, 0.0],
        );

        let speed = 8.1;
        assert!(controller.set_movement_speed_no_ecs(speed));
        assert_abs_diff_eq!(
            controller.compute_local_velocity(),
            vector![0.0, -speed, 0.0],
        );

        let speed = -0.1;
        assert!(controller.set_movement_speed_no_ecs(speed));
        assert_abs_diff_eq!(
            controller.compute_local_velocity(),
            vector![0.0, -speed, 0.0],
        );

        assert!(!controller.set_movement_speed_no_ecs(speed));

        assert!(controller.set_movement_speed_no_ecs(0.0));
        assert_eq!(controller.compute_local_velocity(), Velocity::zeros());
    }
}
