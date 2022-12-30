//! Controllers for user interaction.

mod components;
mod motion;
mod orientation;

pub use components::Controllable;
pub use motion::{
    MotionDirection, MotionState, NoMotionController, SemiDirectionalMotionController,
};
pub use orientation::{CameraOrientationController, NoOrientationController};

use crate::{
    physics::{fph, Orientation, OrientationComp, Velocity, VelocityComp},
    window::Window,
};
use impact_ecs::{query, world::World as ECSWorld};

/// Represents controllers that are used for controlling
/// the movement of entities.
pub trait MotionController: Send + Sync + std::fmt::Debug {
    /// Returns the current [`Velocity`] of the controlled entity in its
    /// local coordinate system.
    fn local_velocity(&self) -> Velocity;

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
    /// Determines the change in orientation of the controlled entity based
    /// on the given displacement of the mouse.
    fn determine_orientation_change(
        &self,
        window: &Window,
        mouse_displacement: (f64, f64),
    ) -> Option<Orientation>;
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

/// Sets the world-space velocities of all controlled
/// entities based on the given local velocity and their
/// orientations.
pub fn set_velocities_of_controlled_entities(ecs_world: &ECSWorld, local_velocity: &Velocity) {
    query!(
        ecs_world,
        |velocity: &mut VelocityComp, orientation: &OrientationComp| {
            let world_velocity = orientation.0.transform_vector(local_velocity);
            velocity.0 = world_velocity;
        },
        [Controllable]
    );
}

/// Updates the orientations of all controlled entities
/// with the given orientation change (the change is
/// appended to the existing orientation).
pub fn update_orientations_of_controlled_entities(
    ecs_world: &ECSWorld,
    orientation_change: &Orientation,
) {
    query!(
        ecs_world,
        |orientation: &mut OrientationComp| {
            orientation.0 *= orientation_change;
        },
        [Controllable]
    );
}
