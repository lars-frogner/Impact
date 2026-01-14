//! Orientation controller implementations.

use super::OrientationController;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    angle::{Angle, Degrees},
    consts::f32::PI,
    quaternion::UnitQuaternion,
    vector::UnitVector3,
};
use impact_physics::quantities::{AngularVelocityP, Orientation};
use roc_integration::roc;

define_component_type! {
    /// Angular velocity controller by a user.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ControlledAngularVelocity(AngularVelocityP);
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
    horizontal_change: Orientation,
    vertical_change: Orientation,
    orientation_has_changed: bool,
    config: RollFreeCameraOrientationControllerConfig,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum OrientationControllerConfig {
    None,
    Camera,
    RollFreeCamera(RollFreeCameraOrientationControllerConfig),
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct RollFreeCameraOrientationControllerConfig {
    pub max_angle_left: Degrees,
    pub max_angle_right: Degrees,
    pub max_angle_down: Degrees,
    pub max_angle_up: Degrees,
}

#[roc]
impl ControlledAngularVelocity {
    /// Creates a new controlled angular velocity.
    #[roc(body = "(Physics.AngularVelocity.zero({}),)")]
    pub fn new() -> Self {
        Self(AngularVelocityP::zero())
    }

    pub fn set_angular_velocity(&mut self, angular_velocity: AngularVelocityP) {
        self.0 = angular_velocity;
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
        *orientation *= self.orientation_change;
    }

    fn orientation_has_changed(&self) -> bool {
        self.orientation_has_changed
    }

    fn update_orientation_change(&mut self, delta_x: f32, delta_y: f32) {
        self.orientation_change = self.orientation_change
            * compute_vertical_rotation(delta_y)
            * compute_horizontal_rotation(-delta_x);

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
    pub fn new(config: RollFreeCameraOrientationControllerConfig) -> Self {
        Self {
            vertical_change: Orientation::identity(),
            horizontal_change: Orientation::identity(),
            orientation_has_changed: false,
            config,
        }
    }

    fn clamp_orientation(&self, orientation: &mut Orientation) {
        let (mut horizontal_angle, mut vertical_angle, _) = orientation.euler_angles_intrinsic();

        // The horizontal angle is in [-PI, PI], with zero being opposite the
        // view direction
        let max_neg_horizontal_angle = -PI + self.config.max_angle_left.radians();
        let min_pos_horizontal_angle = PI - self.config.max_angle_right.radians();

        // The vertical angle is in [-PI/2, PI/2]
        let min_neg_vertical_angle = -self.config.max_angle_down.radians();
        let max_pos_vertical_angle = self.config.max_angle_up.radians();

        if horizontal_angle.is_sign_negative() && horizontal_angle > max_neg_horizontal_angle {
            horizontal_angle = max_neg_horizontal_angle;
        }
        if horizontal_angle.is_sign_positive() && horizontal_angle < min_pos_horizontal_angle {
            horizontal_angle = min_pos_horizontal_angle;
        }
        if vertical_angle.is_sign_negative() && vertical_angle < min_neg_vertical_angle {
            vertical_angle = min_neg_vertical_angle;
        }
        if vertical_angle.is_sign_positive() && vertical_angle > max_pos_vertical_angle {
            vertical_angle = max_pos_vertical_angle;
        }

        // Remove any accumulated roll
        let roll_angle = 0.0;

        *orientation =
            Orientation::from_euler_angles_intrinsic(horizontal_angle, vertical_angle, roll_angle);
    }
}

impl OrientationController for RollFreeCameraOrientationController {
    fn update_orientation(&self, orientation: &mut Orientation) {
        *orientation = self.horizontal_change * (*orientation) * self.vertical_change;
        self.clamp_orientation(orientation);
    }

    fn orientation_has_changed(&self) -> bool {
        self.orientation_has_changed
    }

    fn update_orientation_change(&mut self, delta_x: f32, delta_y: f32) {
        self.horizontal_change = compute_horizontal_rotation(-delta_x) * self.horizontal_change;
        self.vertical_change *= compute_vertical_rotation(delta_y);
        self.orientation_has_changed = true;
    }

    fn reset_orientation_change(&mut self) {
        self.horizontal_change = Orientation::identity();
        self.vertical_change = Orientation::identity();
        self.orientation_has_changed = false;
    }
}

impl Default for OrientationControllerConfig {
    fn default() -> Self {
        Self::RollFreeCamera(Default::default())
    }
}

impl Default for RollFreeCameraOrientationControllerConfig {
    fn default() -> Self {
        Self {
            max_angle_left: Degrees(180.0),
            max_angle_right: Degrees(180.0),
            max_angle_down: Degrees(90.0),
            max_angle_up: Degrees(90.0),
        }
    }
}

fn compute_horizontal_rotation(angular_displacement_x: f32) -> Orientation {
    UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), angular_displacement_x)
}

fn compute_vertical_rotation(angular_displacement_y: f32) -> Orientation {
    UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), angular_displacement_y)
}
