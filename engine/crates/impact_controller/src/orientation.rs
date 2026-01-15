//! Orientation controller implementations.

use super::OrientationController;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    angle::{Angle, Degrees},
    consts::f32::PI,
    quaternion::UnitQuaternion,
    vector::UnitVector3,
};
use impact_physics::quantities::{AngularVelocity, AngularVelocityP, Orientation, OrientationP};
use roc_integration::roc;

define_component_type! {
    /// User control of angular velocity.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct AngularVelocityControl {
        /// The orientation of the reference frame in which the controls should
        /// be applied. This maps the local control directions to world-space
        /// directions.
        pub frame_orientation: OrientationP,
        /// Restrict control to these directions for applicable controllers.
        pub directions: AngularVelocityControlDirections,
        /// The current angular velocity due to the controller.
        controlled_angular_velocity: AngularVelocityP,
    }
}

define_component_type! {
    /// A parent entity whose reference frame the angular velocity controls
    /// should be applied in.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct AngularVelocityControlParent {
        pub entity_id: impact_ecs::world::EntityID,
    }
}

bitflags! {
    /// Directions in which angular velocity can be controlled.
    #[roc(parents="Control", category="bitflags", flags=[HORIZONTAL=0, VERTICAL=1])]
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
    pub struct AngularVelocityControlDirections: u32 {
        const HORIZONTAL = 1 << 0;
        const VERTICAL   = 1 << 1;
    }
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
    horizontal_angle_change: f32,
    vertical_angle_change: f32,
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
impl AngularVelocityControl {
    #[roc(body = r#"
    {
        frame_orientation: UnitQuaternion.identity,
        directions,
        controlled_angular_velocity: Physics.AngularVelocity.zero({}),
    }"#)]
    pub fn new(directions: AngularVelocityControlDirections) -> Self {
        Self {
            frame_orientation: OrientationP::identity(),
            directions,
            controlled_angular_velocity: AngularVelocityP::zero(),
        }
    }

    #[roc(body = r#"
    {
        frame_orientation,
        directions,
        controlled_angular_velocity: Physics.AngularVelocity.zero({}),
    }"#)]
    pub fn new_local(
        frame_orientation: OrientationP,
        directions: AngularVelocityControlDirections,
    ) -> Self {
        Self {
            frame_orientation,
            directions,
            controlled_angular_velocity: AngularVelocityP::zero(),
        }
    }

    pub fn update_orientation(
        &self,
        orientation_controller: &(impl OrientationController + ?Sized),
        orientation: &mut Orientation,
    ) {
        let frame_orientation = self.frame_orientation.unpack();

        let mut orientation_in_local_frame = frame_orientation.inverse() * *orientation;

        orientation_controller.update_orientation(&mut orientation_in_local_frame, self.directions);

        *orientation = frame_orientation * orientation_in_local_frame;
    }

    /// Assigns a new controlled angular velocity and updates the given total
    /// angular velocity to account for the change in controlled angular
    /// velocity.
    pub fn apply_new_controlled_angular_velocity(
        &mut self,
        new_control_angular_velocity: AngularVelocity,
        total_angular_velocity: &mut AngularVelocity,
    ) {
        *total_angular_velocity = &*total_angular_velocity
            - self.controlled_angular_velocity.unpack()
            + &new_control_angular_velocity;

        self.controlled_angular_velocity = new_control_angular_velocity.pack();
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
    fn update_orientation(
        &self,
        orientation: &mut Orientation,
        _directions: AngularVelocityControlDirections,
    ) {
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
            vertical_angle_change: 0.0,
            horizontal_angle_change: 0.0,
            orientation_has_changed: false,
            config,
        }
    }
}

impl OrientationController for RollFreeCameraOrientationController {
    fn update_orientation(
        &self,
        orientation: &mut Orientation,
        directions: AngularVelocityControlDirections,
    ) {
        let (mut horizontal_angle, mut vertical_angle, _) = orientation.euler_angles_intrinsic();

        if directions.contains(AngularVelocityControlDirections::HORIZONTAL) {
            horizontal_angle += self.horizontal_angle_change;

            // The horizontal angle is in [-PI, PI], with zero being opposite the
            // view direction
            let max_neg_horizontal_angle = -PI + self.config.max_angle_left.radians();
            let min_pos_horizontal_angle = PI - self.config.max_angle_right.radians();

            if horizontal_angle.is_sign_negative() && horizontal_angle > max_neg_horizontal_angle {
                horizontal_angle = max_neg_horizontal_angle;
            }
            if horizontal_angle.is_sign_positive() && horizontal_angle < min_pos_horizontal_angle {
                horizontal_angle = min_pos_horizontal_angle;
            }
        }

        if directions.contains(AngularVelocityControlDirections::VERTICAL) {
            vertical_angle += self.vertical_angle_change;

            // The vertical angle is in [-PI/2, PI/2]
            let min_neg_vertical_angle = -self.config.max_angle_down.radians();
            let max_pos_vertical_angle = self.config.max_angle_up.radians();

            if vertical_angle.is_sign_negative() && vertical_angle < min_neg_vertical_angle {
                vertical_angle = min_neg_vertical_angle;
            }
            if vertical_angle.is_sign_positive() && vertical_angle > max_pos_vertical_angle {
                vertical_angle = max_pos_vertical_angle;
            }
        }

        *orientation =
            Orientation::from_euler_angles_intrinsic(horizontal_angle, vertical_angle, 0.0);
    }

    fn orientation_has_changed(&self) -> bool {
        self.orientation_has_changed
    }

    fn update_orientation_change(&mut self, delta_x: f32, delta_y: f32) {
        self.horizontal_angle_change -= delta_x;
        self.vertical_angle_change += delta_y;
        self.orientation_has_changed = true;
    }

    fn reset_orientation_change(&mut self) {
        self.horizontal_angle_change = 0.0;
        self.vertical_angle_change = 0.0;
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
