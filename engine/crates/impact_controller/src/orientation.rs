//! Orientation controller implementations.

use super::OrientationController;
use bytemuck::{Pod, Zeroable};
use impact_math::quaternion::UnitQuaternion;
use impact_physics::quantities::{AngularVelocity, Orientation};
use nalgebra::Vector3;
use roc_integration::roc;

define_component_type! {
    /// Angular velocity controller by a user.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ControlledAngularVelocity(AngularVelocity);
}

/// Orientation controller that updates the orientation in the way a
/// first-person camera should respond to mouse movement.
///
/// Orienting the camera may introduce roll (viewed objects may not stay
/// upright).
#[derive(Clone, Debug)]
pub struct CameraOrientationController {
    orientation_change: Orientation,
    orientation_has_changed: bool,
}

/// Orientation controller that updates the orientation in the way a
/// first-person camera should respond to mouse movement.
///
/// Orienting the camera will not introduce any roll (viewed objects remain
/// upright), but orientation only remains correct when the camera stays in the
/// world's horizontal plane.
#[derive(Clone, Debug)]
pub struct RollFreeCameraOrientationController {
    yaw_change: Orientation,
    pitch_change: Orientation,
    orientation_has_changed: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub enum OrientationControllerConfig {
    None,
    Camera,
    #[default]
    RollFreeCamera,
}

#[roc]
impl ControlledAngularVelocity {
    /// Creates a new controlled angular velocity.
    #[roc(body = "(Physics.AngularVelocity.zero({}),)")]
    pub fn new() -> Self {
        Self(AngularVelocity::zero())
    }

    /// Assigns a new controlled angular velocity and updates the given total
    /// angular velocity to account for the change in controlled angular
    /// velocity.
    pub fn apply_new_controlled_angular_velocity(
        &mut self,
        new_control_angular_velocity: AngularVelocity,
        total_angular_velocity: &mut AngularVelocity,
    ) {
        *total_angular_velocity -= self.0;
        *total_angular_velocity += new_control_angular_velocity;
        self.0 = new_control_angular_velocity;
    }
}

impl Default for ControlledAngularVelocity {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraOrientationController {
    /// Creates a new first-person camera orientation controller.
    pub fn new() -> Self {
        Self {
            orientation_change: Orientation::identity(),
            orientation_has_changed: false,
        }
    }
}

impl OrientationController for CameraOrientationController {
    fn update_orientation(&self, orientation: &mut Orientation) {
        *orientation = *orientation * self.orientation_change;
    }

    fn orientation_has_changed(&self) -> bool {
        self.orientation_has_changed
    }

    fn update_orientation_change(&mut self, delta_x: f32, delta_y: f32) {
        self.orientation_change = self.orientation_change
            * compute_pitch_rotation(delta_y)
            * compute_yaw_rotation(-delta_x);

        self.orientation_has_changed = true;
    }

    fn reset_orientation_change(&mut self) {
        self.orientation_change = Orientation::identity();
        self.orientation_has_changed = false;
    }
}

impl Default for CameraOrientationController {
    fn default() -> Self {
        Self::new()
    }
}

impl RollFreeCameraOrientationController {
    /// Creates a new roll-free first-person camera orientation controller.
    pub fn new() -> Self {
        Self {
            pitch_change: Orientation::identity(),
            yaw_change: Orientation::identity(),
            orientation_has_changed: false,
        }
    }
}

impl OrientationController for RollFreeCameraOrientationController {
    fn update_orientation(&self, orientation: &mut Orientation) {
        *orientation = self.yaw_change * (*orientation) * self.pitch_change;
    }

    fn orientation_has_changed(&self) -> bool {
        self.orientation_has_changed
    }

    fn update_orientation_change(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw_change = compute_yaw_rotation(-delta_x) * self.yaw_change;
        self.pitch_change = self.pitch_change * compute_pitch_rotation(delta_y);
        self.orientation_has_changed = true;
    }

    fn reset_orientation_change(&mut self) {
        self.yaw_change = Orientation::identity();
        self.pitch_change = Orientation::identity();
        self.orientation_has_changed = false;
    }
}

impl Default for RollFreeCameraOrientationController {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_yaw_rotation(angular_displacement_x: f32) -> Orientation {
    UnitQuaternion::from_axis_angle(&Vector3::y_axis(), angular_displacement_x)
}

fn compute_pitch_rotation(angular_displacement_y: f32) -> Orientation {
    UnitQuaternion::from_axis_angle(&Vector3::x_axis(), angular_displacement_y)
}
