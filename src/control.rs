//! Controllers for user interaction.

mod components;
mod motion;
mod orientation;

pub use components::{register_control_components, MotionControlComp, OrientationControlComp};
pub use motion::{MotionDirection, MotionState, SemiDirectionalMotionController};
pub use orientation::{CameraOrientationController, RollFreeCameraOrientationController};

use crate::{
    physics::{
        fph, AngularVelocity, AngularVelocityComp, Orientation, RigidBodyComp,
        SpatialConfigurationComp, Velocity, VelocityComp,
    },
    window::Window,
};
use impact_ecs::{query, world::World as ECSWorld};

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

/// Updates the world-space velocities of all entities controlled by the given
/// motion controller, and advances their positions by the given time step
/// duration when applicable.
pub fn update_motion_of_controlled_entities(
    ecs_world: &ECSWorld,
    motion_controller: &(impl MotionController + ?Sized),
    time_step_duration: fph,
) {
    query!(
        ecs_world,
        |motion_control: &mut MotionControlComp,
         velocity: &mut VelocityComp,
         spatial: &mut SpatialConfigurationComp| {
            let new_control_velocity =
                motion_controller.compute_control_velocity(&spatial.orientation);
            motion_control.apply_new_control_velocity(new_control_velocity, &mut velocity.0);

            spatial.position += velocity.0 * time_step_duration;
        },
        ![RigidBodyComp]
    );
    query!(
        ecs_world,
        |motion_control: &mut MotionControlComp,
         rigid_body: &mut RigidBodyComp,
         velocity: &mut VelocityComp,
         spatial: &SpatialConfigurationComp| {
            let new_control_velocity =
                motion_controller.compute_control_velocity(&spatial.orientation);
            motion_control.apply_new_control_velocity(new_control_velocity, &mut velocity.0);

            rigid_body.0.synchronize_momentum(&velocity.0);
        }
    );
}

/// Updates the angular velocities and/or orientations of all entities
/// controlled by the given orientation controller.
pub fn update_rotation_of_controlled_entities(
    ecs_world: &ECSWorld,
    orientation_controller: &mut (impl OrientationController + ?Sized),
    time_step_duration: fph,
) {
    if orientation_controller.orientation_has_changed() {
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp| {
                orientation_controller.update_orientation(&mut spatial.orientation);
            },
            [OrientationControlComp],
            ![AngularVelocityComp, RigidBodyComp]
        );
    }
    query!(
        ecs_world,
        |orientation_control: &mut OrientationControlComp,
         spatial: &mut SpatialConfigurationComp,
         angular_velocity: &mut AngularVelocityComp| {
            let new_control_angular_velocity = if orientation_controller.orientation_has_changed() {
                let old_orientation = spatial.orientation.clone();
                orientation_controller.update_orientation(&mut spatial.orientation);

                AngularVelocity::from_consecutive_orientations(
                    &old_orientation,
                    &spatial.orientation,
                    time_step_duration,
                )
            } else {
                AngularVelocity::zero()
            };

            orientation_control.apply_new_control_angular_velocity(
                new_control_angular_velocity,
                &mut angular_velocity.0,
            );
        },
        ![RigidBodyComp]
    );
    query!(
        ecs_world,
        |orientation_control: &mut OrientationControlComp,
         rigid_body: &mut RigidBodyComp,
         spatial: &SpatialConfigurationComp,
         angular_velocity: &mut AngularVelocityComp| {
            let new_control_angular_velocity = if orientation_controller.orientation_has_changed() {
                // We do not update the orientation here, as the rigid body
                // motion system will handle that for us as long as we apply the
                // correct angular velocity
                let mut new_orientation = spatial.orientation.clone();
                orientation_controller.update_orientation(&mut new_orientation);

                AngularVelocity::from_consecutive_orientations(
                    &spatial.orientation,
                    &new_orientation,
                    time_step_duration,
                )
            } else {
                AngularVelocity::zero()
            };

            orientation_control.apply_new_control_angular_velocity(
                new_control_angular_velocity,
                &mut angular_velocity.0,
            );

            rigid_body
                .0
                .synchronize_angular_momentum(&spatial.orientation, &angular_velocity.0);
        }
    );

    orientation_controller.reset_orientation_change();
}
