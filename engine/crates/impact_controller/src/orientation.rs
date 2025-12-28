//! Orientation controller implementations.

use super::OrientationController;
use bytemuck::{Pod, Zeroable};
use impact_math::{quaternion::UnitQuaternionA, vector::UnitVector3A};
use impact_physics::quantities::{AngularVelocity, AngularVelocityA, OrientationA};
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
    orientation_change: OrientationA,
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
    yaw_change: OrientationA,
    pitch_change: OrientationA,
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
        new_control_angular_velocity: AngularVelocityA,
        total_angular_velocity: &mut AngularVelocityA,
    ) {
        *total_angular_velocity =
            &*total_angular_velocity - self.0.aligned() + &new_control_angular_velocity;
        self.0 = new_control_angular_velocity.unaligned();
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
            orientation_change: OrientationA::identity(),
            orientation_has_changed: false,
        }
    }
}

impl OrientationController for CameraOrientationController {
    fn update_orientation(&self, orientation: &mut OrientationA) {
        *orientation *= self.orientation_change;
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
        self.orientation_change = OrientationA::identity();
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
            pitch_change: OrientationA::identity(),
            yaw_change: OrientationA::identity(),
            orientation_has_changed: false,
        }
    }
}

impl OrientationController for RollFreeCameraOrientationController {
    fn update_orientation(&self, orientation: &mut OrientationA) {
        *orientation = self.yaw_change * (*orientation) * self.pitch_change;
    }

    fn orientation_has_changed(&self) -> bool {
        self.orientation_has_changed
    }

    fn update_orientation_change(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw_change = compute_yaw_rotation(-delta_x) * self.yaw_change;
        self.pitch_change *= compute_pitch_rotation(delta_y);
        self.orientation_has_changed = true;
    }

    fn reset_orientation_change(&mut self) {
        self.yaw_change = OrientationA::identity();
        self.pitch_change = OrientationA::identity();
        self.orientation_has_changed = false;
    }
}

impl Default for RollFreeCameraOrientationController {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_yaw_rotation(angular_displacement_x: f32) -> OrientationA {
    UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_y(), angular_displacement_x)
}

fn compute_pitch_rotation(angular_displacement_y: f32) -> OrientationA {
    UnitQuaternionA::from_axis_angle(&UnitVector3A::unit_x(), angular_displacement_y)
}
