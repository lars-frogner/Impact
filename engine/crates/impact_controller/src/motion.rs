//! Motion controller implementations.

use super::{MotionChanged, MotionController};
use approx::{abs_diff_eq, assert_abs_diff_ne};
use bytemuck::{Pod, Zeroable};
use impact_physics::quantities::{Orientation, Velocity, VelocityP};
use roc_integration::roc;

define_component_type! {
    /// Velocity controller by a user.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ControlledVelocity(VelocityP);
}

/// Motion controller allowing for motion at constant speed along the axes of an
/// entity's local coordinate system (`W-A-S-D` type motion).
#[derive(Clone, Debug)]
pub struct SemiDirectionalMotionController {
    movement_speed: f32,
    vertical_control: bool,
    state: SemiDirectionalMotionState,
    local_velocity: Velocity,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum MotionControllerConfig {
    None,
    SemiDirectional(SemiDirectionalMotionControllerConfig),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct SemiDirectionalMotionControllerConfig {
    /// The speed at which the controlled entity can move.
    pub movement_speed: f32,
    /// Whether the controller can move the entity in the vertical direction.
    pub vertical_control: bool,
}

/// Whether there is motion in a certain direction.
#[roc(parents = "Control")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionState {
    Still,
    Moving,
}

/// Possible directions of motion in the local coordinate system.
#[roc(parents = "Control")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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

#[roc]
impl ControlledVelocity {
    /// Creates a new controlled velocity.
    #[roc(body = "(Vector3.zero,)")]
    pub fn new() -> Self {
        Self(VelocityP::zeros())
    }

    /// Assigns a new controlled velocity and updates the given total velocity
    /// to account for the change in controlled velocity.
    pub fn apply_new_controlled_velocity(
        &mut self,
        new_controlled_velocity: Velocity,
        total_velocity: &mut Velocity,
    ) {
        *total_velocity = *total_velocity - self.0.unpack() + new_controlled_velocity;
        self.0 = new_controlled_velocity.pack();
    }
}

impl Default for ControlledVelocity {
    fn default() -> Self {
        Self::new()
    }
}

impl SemiDirectionalMotionController {
    /// Creates a new motion controller with the given configuration parameters.
    pub fn new(config: SemiDirectionalMotionControllerConfig) -> Self {
        Self {
            movement_speed: config.movement_speed,
            vertical_control: config.vertical_control,
            state: SemiDirectionalMotionState::new(),
            local_velocity: Velocity::zeros(),
        }
    }

    /// Computes the velocity of the controlled entity in its local coordinate
    /// system.
    fn compute_local_velocity(&self) -> Velocity {
        if self.state.motion_state() == MotionState::Still || abs_diff_eq!(self.movement_speed, 0.0)
        {
            Velocity::zeros()
        } else {
            // For scaling the magnitude to unity
            let mut n_nonzero_components = 0.0;

            let velocity_x = if self.state.right == self.state.left {
                0.0
            } else {
                n_nonzero_components += 1.0;
                if self.state.right.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            let velocity_y = if self.state.up == self.state.down {
                0.0
            } else {
                n_nonzero_components += 1.0;
                if self.state.up.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            let velocity_z = if self.state.forwards == self.state.backwards {
                0.0
            } else {
                n_nonzero_components += 1.0;
                if self.state.forwards.is_moving() {
                    self.movement_speed
                } else {
                    -self.movement_speed
                }
            };

            // We should have motion in this branch
            assert_abs_diff_ne!(n_nonzero_components, 0.0);

            let magnitude_scale = 1.0 / f32::sqrt(n_nonzero_components);

            Velocity::new(velocity_x, velocity_y, velocity_z) * magnitude_scale
        }
    }
}

impl MotionController for SemiDirectionalMotionController {
    fn movement_speed(&self) -> f32 {
        self.movement_speed
    }

    fn compute_controlled_velocity(&self, orientation: &Orientation) -> Velocity {
        let mut controlled_velocity = orientation.rotate_vector(&self.local_velocity);
        if !self.vertical_control {
            *controlled_velocity.y_mut() = 0.0;
        }
        controlled_velocity
    }

    fn update_motion(&mut self, state: MotionState, direction: MotionDirection) -> MotionChanged {
        let result = self.state.update(state, direction);
        if result.motion_changed() {
            self.local_velocity = self.compute_local_velocity();
        }
        result
    }

    fn set_movement_speed(&mut self, movement_speed: f32) -> MotionChanged {
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

impl Default for MotionControllerConfig {
    fn default() -> Self {
        Self::SemiDirectional(SemiDirectionalMotionControllerConfig::default())
    }
}

impl Default for SemiDirectionalMotionControllerConfig {
    fn default() -> Self {
        Self {
            movement_speed: 8.0,
            vertical_control: true,
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
        // This takes into account that motion in oppsite directions will be
        // cancelled out
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
    use MotionDirection::{Backwards, Down, Forwards, Left, Up};
    use MotionState::{Moving, Still};
    use approx::assert_abs_diff_eq;
    use impact_math::consts::f32::SQRT_2;

    #[test]
    fn updating_semi_directional_motion_works() {
        let speed = 1.3;
        let mut controller =
            SemiDirectionalMotionController::new(SemiDirectionalMotionControllerConfig {
                movement_speed: speed,
                vertical_control: false,
            });
        assert_eq!(
            controller.local_velocity,
            Velocity::zeros(),
            "Not stationary directly after initalization"
        );

        assert!(controller.update_motion(Moving, Forwards).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, Velocity::new(0.0, 0.0, speed));

        assert!(!controller.update_motion(Moving, Forwards).motion_changed());

        assert!(controller.update_motion(Moving, Backwards).motion_changed());
        assert_eq!(
            controller.local_velocity,
            Velocity::zeros(),
            "Motion does not cancel"
        );

        assert!(controller.update_motion(Moving, Left).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, Velocity::new(-speed, 0.0, 0.0));

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
            Velocity::new(0.0, speed, -speed) / SQRT_2, // Magnitude should be `speed`
            epsilon = 1e-6
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
        let mut controller =
            SemiDirectionalMotionController::new(SemiDirectionalMotionControllerConfig {
                movement_speed: speed,
                vertical_control: false,
            });

        controller.update_motion(Moving, Down);
        assert_abs_diff_eq!(controller.local_velocity, Velocity::new(0.0, -speed, 0.0));

        let speed = 8.1;
        assert!(controller.set_movement_speed(speed).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, Velocity::new(0.0, -speed, 0.0));

        let speed = -0.1;
        assert!(controller.set_movement_speed(speed).motion_changed());
        assert_abs_diff_eq!(controller.local_velocity, Velocity::new(0.0, -speed, 0.0));

        assert!(!controller.set_movement_speed(speed).motion_changed());

        assert!(controller.set_movement_speed(0.0).motion_changed());
        assert_eq!(controller.local_velocity, Velocity::zeros());
    }
}
