//! Controllers for user interaction.

mod motion;

pub use motion::{
    MotionDirection, MotionState, NoMotionController, SemiDirectionalMotionController,
};

use nalgebra::{Rotation3, Translation3};

/// Represents controllers that are used for controlling
/// the movement of entities.
pub trait MotionController<F>: Send + Sync + std::fmt::Debug {
    fn next_translation(&mut self) -> Option<Translation3<F>>;

    /// Specifies whether the controlled entity should be moving in
    /// a given direction.
    fn update_motion(&mut self, state: MotionState, direction: MotionDirection);

    /// Specifies how the local coordinate system of the
    /// controlled entity is oriented.
    fn set_orientation(&mut self, orientation: Rotation3<F>);

    /// Rotates the orientation of the local coordinate system.
    fn rotate_orientation(&mut self, rotation: &Rotation3<F>);

    /// Specifies how fast the controlled entity should be moving when
    /// in motion.
    fn set_movement_speed(&mut self, movement_speed: F);

    /// Stops any motion of the controlled entity.
    fn stop(&mut self);
}
