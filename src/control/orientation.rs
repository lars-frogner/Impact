//! Orientation controller implementations.

use super::OrientationController;
use crate::{
    geometry::{Angle, Radians},
    physics::{fph, Orientation},
    window::Window,
};
use nalgebra::{UnitQuaternion, Vector3};

/// Orientation controller that allows no control over
/// orientation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoOrientationController;

/// Orientation controller that updates the orientation
/// in the way a first-person camera should respond to
/// mouse movement.
#[derive(Clone, Debug)]
pub struct CameraOrientationController {
    vertical_field_of_view: Radians<f64>,
    sensitivity: f64,
}

impl CameraOrientationController {
    /// Creates a new orientation controller for a first-person
    /// camera with the given vertical field of view, with the
    /// given sensitivity to mouse motions.
    pub fn new<A: Angle<f64>>(vertical_field_of_view: A, sensitivity: f64) -> Self {
        Self {
            vertical_field_of_view: vertical_field_of_view.as_radians(),
            sensitivity,
        }
    }
}

impl OrientationController for NoOrientationController {
    fn determine_orientation_change(
        &self,
        _window: &Window,
        _mouse_displacement: (f64, f64),
    ) -> Option<Orientation> {
        None
    }
}

impl OrientationController for CameraOrientationController {
    fn determine_orientation_change(
        &self,
        window: &Window,
        mouse_displacement: (f64, f64),
    ) -> Option<Orientation> {
        let (_, height) = window.dimensions();
        let degrees_per_pixel = self.vertical_field_of_view / (height as f64);

        let offset_x = degrees_per_pixel * mouse_displacement.0 * self.sensitivity;
        let offset_y = degrees_per_pixel * (-mouse_displacement.1) * self.sensitivity;

        let orientation_change =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), offset_y.radians() as fph)
                * UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -offset_x.radians() as fph);

        Some(orientation_change)
    }
}
