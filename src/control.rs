//! Controllers for user interaction.

pub mod motion;
pub mod orientation;

use crate::{
    physics::{
        fph,
        motion::{Orientation, Velocity},
    },
    window::Window,
};
use motion::{
    MotionControllerConfig, MotionDirection, MotionState, SemiDirectionalMotionController,
};
use orientation::{
    CameraOrientationController, OrientationControllerConfig, RollFreeCameraOrientationController,
};
use serde::{Deserialize, Serialize};

/// Represents controllers that are used for controlling
/// the movement of entities.
pub trait MotionController: Send + Sync + std::fmt::Debug {
    /// Returns the current movement speed.
    fn movement_speed(&self) -> fph;

    /// Computes the world space velocity that should be added to the controlled
    /// entity's velocity when in motion.
    fn compute_control_velocity(&self, orientation: &Orientation) -> Velocity;

    /// Updates the overall motion state of the controlled entity based on the
    /// given [`MotionState`] specifying whether the entity should be moving
    /// in the given [`MotionDirection`] in its local coordinate system.
    ///
    /// # Returns
    /// An enum indicating whether the update caused the local velocity to
    /// change.
    fn update_motion(&mut self, state: MotionState, direction: MotionDirection) -> MotionChanged;

    /// Updates the speed that should be added to the controlled entity's speed
    /// when in motion.
    ///
    /// # Returns
    /// An enum indicating whether the update caused the local velocity to
    /// change.
    fn set_movement_speed(&mut self, movement_speed: fph) -> MotionChanged;

    /// Stops the controlled motion of the entity.
    ///
    /// # Returns
    /// An enum indicating whether the update caused the local velocity to
    /// change.
    fn stop(&mut self) -> MotionChanged;
}

/// Represents controllers that are used for controlling
/// the orientation of entities.
pub trait OrientationController: Send + Sync + std::fmt::Debug {
    /// Returns the sensitivity of the controller.
    fn sensitivity(&self) -> f64;

    /// Modifies the given orientation of a controlled entity so that the
    /// current changes in orientation are applied to it.
    fn update_orientation(&self, orientation: &mut Orientation);

    /// Whether the orientation has changed since calling
    /// [`reset_orientation_change`](Self::reset_orientation_change).
    fn orientation_has_changed(&self) -> bool;

    /// Determines and registers the change in orientation of the
    /// controlled entity based on the given displacement of the mouse.
    fn update_orientation_change(&mut self, window: &Window, mouse_displacement: (f64, f64));

    /// Resets the change in orientation accumulated by
    /// [`update_orientation_change`](Self::update_orientation_change).
    fn reset_orientation_change(&mut self);

    /// Sets the given sensitivity for the controller.
    ///
    /// # Panics
    /// If the given sensitivity does not exceed zero.
    fn set_sensitivity(&mut self, sensitivity: f64);
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ControllerConfig {
    pub motion: MotionControllerConfig,
    pub orientation: OrientationControllerConfig,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MotionChanged {
    Yes,
    No,
}

impl MotionChanged {
    pub fn motion_changed(&self) -> bool {
        *self == Self::Yes
    }
}

#[allow(clippy::type_complexity)]
pub fn create_controllers(
    ControllerConfig {
        motion: motion_config,
        orientation: orientation_config,
    }: ControllerConfig,
) -> (
    Option<Box<dyn MotionController>>,
    Option<Box<dyn OrientationController>>,
) {
    let motion_controller: Option<Box<dyn MotionController>> = match motion_config {
        MotionControllerConfig::None => None,
        MotionControllerConfig::SemiDirectional(motion_controller_config) => Some(Box::new(
            SemiDirectionalMotionController::new(motion_controller_config),
        )),
    };

    let orientation_controller: Option<Box<dyn OrientationController>> = match orientation_config {
        OrientationControllerConfig::None => None,
        OrientationControllerConfig::Camera(camera_orientation_controller_config) => {
            Some(Box::new(CameraOrientationController::new(
                camera_orientation_controller_config,
            )))
        }
        OrientationControllerConfig::RollFreeCamera(camera_orientation_controller_config) => {
            Some(Box::new(RollFreeCameraOrientationController::new(
                camera_orientation_controller_config,
            )))
        }
    };

    (motion_controller, orientation_controller)
}
