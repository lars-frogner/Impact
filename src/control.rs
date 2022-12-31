//! Controllers for user interaction.

mod components;
mod motion;
mod orientation;

pub use components::Controllable;
pub use motion::{MotionDirection, MotionState, SemiDirectionalMotionController};
pub use orientation::{CameraOrientationController, RollFreeCameraOrientationController};

use crate::{
    physics::{fph, Orientation, OrientationComp, Velocity, VelocityComp},
    window::Window,
};
use impact_ecs::{query, world::World as ECSWorld};

/// Represents controllers that are used for controlling
/// the movement of entities.
pub trait MotionController: Send + Sync + std::fmt::Debug {
    /// Updates the given world-space velocity of a controlled entity
    /// given its orientation.
    fn update_world_velocity(&self, velocity: &mut Velocity, orientation: &Orientation);

    /// Updates the overall motion state of the controlled entity based on the
    /// given [`MotionState`] specifying whether the entity should be moving
    /// in the given [`MotionDirection`] in its local coordinate system.
    ///
    /// # Returns
    /// An enum indicating whether the update caused the local velocity to
    /// change.
    fn update_motion(&mut self, state: MotionState, direction: MotionDirection) -> MotionChanged;

    /// Updates the speed in which the controlled entity should be moving when
    /// in motion.
    ///
    /// # Returns
    /// An enum indicating whether the update caused the local velocity to
    /// change.
    fn set_movement_speed(&mut self, movement_speed: fph) -> MotionChanged;

    /// Stops any motion of the controlled entity.
    ///
    /// # Returns
    /// An enum indicating whether the update caused the local velocity to
    /// change.
    fn stop(&mut self) -> MotionChanged;
}

/// Represents controllers that are used for controlling
/// the orientation of entities.
pub trait OrientationController: Send + Sync + std::fmt::Debug {
    /// Modifies the given orientation of a controlled entity so
    /// that the current changes in orientation are applied to it.
    fn update_orientation(&self, orientation: &mut Orientation);

    /// Determines and registers the change in orientation of the
    /// controlled entity based on the given displacement of the mouse.
    fn update_orientation_change(&mut self, window: &Window, mouse_displacement: (f64, f64));
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

/// Sets the world-space velocities of all entities
/// controlled by the given motion controller.
pub fn set_velocities_of_controlled_entities(
    ecs_world: &ECSWorld,
    motion_controller: &(impl MotionController + ?Sized),
) {
    query!(
        ecs_world,
        |velocity: &mut VelocityComp, orientation: &OrientationComp| {
            motion_controller.update_world_velocity(&mut velocity.0, &orientation.0);
        },
        [Controllable]
    );
}

/// Updates the orientations of all entities controlled
/// by the given orientation controller.
pub fn update_orientations_of_controlled_entities(
    ecs_world: &ECSWorld,
    orientation_controller: &(impl OrientationController + ?Sized),
) {
    query!(
        ecs_world,
        |orientation: &mut OrientationComp| {
            orientation_controller.update_orientation(&mut orientation.0);
        },
        [Controllable]
    );
}
