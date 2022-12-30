//! Controllers for user interaction.

mod components;
mod motion;
mod orientation;

pub use components::Controllable;
pub use motion::{
    MotionDirection, MotionState, NoMotionController, SemiDirectionalMotionController,
};
pub use orientation::{CameraOrientationController, NoOrientationController};

use crate::{physics::fph, window::Window};
use impact_ecs::world::World as ECSWorld;

/// Represents controllers that are used for controlling
/// the movement of entities.
pub trait MotionController: Send + Sync + std::fmt::Debug {
    /// Updates the motion of the controlled entity based on the given
    /// [`MotionState`] specifying whether the entity should be moving
    /// in the given [`MotionDirection`] in its local coordinate system.
    fn update_motion(
        &mut self,
        ecs_world: &ECSWorld,
        state: MotionState,
        direction: MotionDirection,
    );

    /// Updates the speed in which the controlled entity should be moving when
    /// in motion.
    fn set_movement_speed(&mut self, ecs_world: &ECSWorld, movement_speed: fph);

    /// Stops any motion of the controlled entity.
    fn stop(&mut self, ecs_world: &ECSWorld);
}

/// Represents controllers that are used for controlling
/// the orientation of entities.
pub trait OrientationController: Send + Sync + std::fmt::Debug {
    /// Updates the orientation of the controlled entity based on the given
    /// displacement of the mouse.
    fn update_orientation(
        &self,
        window: &Window,
        ecs_world: &ECSWorld,
        mouse_displacement: (f64, f64),
    );
}
