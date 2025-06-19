//! Orientation controller implementations.

pub mod components;
pub mod systems;

use super::OrientationController;
use crate::physics::{fph, motion::Orientation};
use impact_math::{Angle, Degrees, Radians};
use nalgebra::{UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Orientation controller that updates the orientation
/// in the way a first-person camera should respond to
/// mouse movement.
///
/// Orienting the camera may introduce roll (viewed objects
/// may not stay upright).
#[derive(Clone, Debug)]
pub struct CameraOrientationController {
    base: CameraOrientationControllerBase,
    orientation_change: Orientation,
}

/// Orientation controller that updates the orientation
/// in the way a first-person camera should respond to
/// mouse movement.
///
/// Orienting the camera will not introduce any roll
/// (viewed objects remain upright), but orientation
/// only remains correct when the camera stays in the
/// world's horizontal plane.
#[derive(Clone, Debug)]
pub struct RollFreeCameraOrientationController {
    base: CameraOrientationControllerBase,
    yaw_change: Orientation,
    pitch_change: Orientation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrientationControllerConfig {
    None,
    Camera(CameraOrientationControllerConfig),
    RollFreeCamera(CameraOrientationControllerConfig),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraOrientationControllerConfig {
    /// The vertical field of view of the controlled first-person camera.
    pub vertical_field_of_view: Degrees<f64>,
    /// The sensitivity to mouse motions.
    pub sensitivity: f64,
}

#[derive(Clone, Debug)]
struct CameraOrientationControllerBase {
    vertical_field_of_view: Radians<f64>,
    sensitivity: f64,
    orientation_has_changed: bool,
}

impl CameraOrientationController {
    /// Creates a new first-person camera orientation controller with the given
    /// configuration parameters.
    pub fn new(config: CameraOrientationControllerConfig) -> Self {
        Self {
            base: CameraOrientationControllerBase::new(
                config.vertical_field_of_view,
                config.sensitivity,
            ),
            orientation_change: Orientation::identity(),
        }
    }
}

impl RollFreeCameraOrientationController {
    /// Creates a new roll-free first-person camera orientation controller
    /// with the given configuration parameters.
    pub fn new(config: CameraOrientationControllerConfig) -> Self {
        Self {
            base: CameraOrientationControllerBase::new(
                config.vertical_field_of_view,
                config.sensitivity,
            ),
            pitch_change: Orientation::identity(),
            yaw_change: Orientation::identity(),
        }
    }
}

impl OrientationController for CameraOrientationController {
    fn sensitivity(&self) -> f64 {
        self.base.sensitivity
    }

    fn update_orientation(&self, orientation: &mut Orientation) {
        *orientation *= self.orientation_change;
    }

    fn orientation_has_changed(&self) -> bool {
        self.base.orientation_has_changed
    }

    fn update_orientation_change(
        &mut self,
        window_height: NonZeroU32,
        mouse_displacement: (f64, f64),
    ) {
        let (angular_displacement_x, angular_displacement_y) = self
            .base
            .compute_angular_displacements(window_height, mouse_displacement);

        self.orientation_change *=
            CameraOrientationControllerBase::compute_pitch_rotation(angular_displacement_y)
                * CameraOrientationControllerBase::compute_yaw_rotation(angular_displacement_x);

        self.base.orientation_has_changed = true;
    }

    fn reset_orientation_change(&mut self) {
        self.orientation_change = Orientation::identity();
        self.base.orientation_has_changed = false;
    }

    fn set_sensitivity(&mut self, sensitivity: f64) {
        self.base.set_sensitivity(sensitivity);
    }
}

impl OrientationController for RollFreeCameraOrientationController {
    fn sensitivity(&self) -> f64 {
        self.base.sensitivity
    }

    fn update_orientation(&self, orientation: &mut Orientation) {
        *orientation = self.yaw_change * (*orientation) * self.pitch_change;
    }

    fn orientation_has_changed(&self) -> bool {
        self.base.orientation_has_changed
    }

    fn update_orientation_change(
        &mut self,
        window_height: NonZeroU32,
        mouse_displacement: (f64, f64),
    ) {
        let (angular_displacement_x, angular_displacement_y) = self
            .base
            .compute_angular_displacements(window_height, mouse_displacement);

        self.yaw_change =
            CameraOrientationControllerBase::compute_yaw_rotation(angular_displacement_x)
                * self.yaw_change;
        self.pitch_change *=
            CameraOrientationControllerBase::compute_pitch_rotation(angular_displacement_y);

        self.base.orientation_has_changed = true;
    }

    fn reset_orientation_change(&mut self) {
        self.yaw_change = Orientation::identity();
        self.pitch_change = Orientation::identity();
        self.base.orientation_has_changed = false;
    }

    fn set_sensitivity(&mut self, sensitivity: f64) {
        self.base.set_sensitivity(sensitivity);
    }
}

impl Default for OrientationControllerConfig {
    fn default() -> Self {
        Self::RollFreeCamera(CameraOrientationControllerConfig::default())
    }
}

impl Default for CameraOrientationControllerConfig {
    fn default() -> Self {
        Self {
            vertical_field_of_view: Degrees(70.0),
            sensitivity: 1.0,
        }
    }
}

impl CameraOrientationControllerBase {
    fn new<A: Angle<f64>>(vertical_field_of_view: A, sensitivity: f64) -> Self {
        Self {
            vertical_field_of_view: vertical_field_of_view.as_radians(),
            sensitivity,
            orientation_has_changed: false,
        }
    }

    fn compute_angular_displacements(
        &self,
        window_height: NonZeroU32,
        mouse_displacement: (f64, f64),
    ) -> (Radians<f64>, Radians<f64>) {
        let degrees_per_pixel = self.vertical_field_of_view / f64::from(u32::from(window_height));

        let angular_displacement_x = degrees_per_pixel * (-mouse_displacement.0) * self.sensitivity;
        let angular_displacement_y = degrees_per_pixel * (-mouse_displacement.1) * self.sensitivity;

        (angular_displacement_x, angular_displacement_y)
    }

    fn set_sensitivity(&mut self, sensitivity: f64) {
        assert!(
            sensitivity > 0.0,
            "Tried to set orientation controller sensitivity not exceeding zero"
        );
        self.sensitivity = sensitivity;
    }

    fn compute_yaw_rotation(angular_displacement_x: Radians<f64>) -> Orientation {
        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), angular_displacement_x.radians() as fph)
    }

    fn compute_pitch_rotation(angular_displacement_y: Radians<f64>) -> Orientation {
        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), angular_displacement_y.radians() as fph)
    }
}
