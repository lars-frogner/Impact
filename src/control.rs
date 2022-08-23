//! Controllers for user interaction.

mod motion;

pub use motion::{
    MotionDirection, MotionState, NoMotionController, SemiDirectionalMotionController,
};

use crate::num::Float;
use nalgebra::{Rotation3, Vector3};

/// Represents controllers that are used for controlling
/// the movement of entities.
pub trait MotionController<F: Float>: Send + Sync + std::fmt::Debug {
    /// Returns the current motion of the controlled entity.
    fn current_motion(&self) -> &ControlledMotion<F>;

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

/// Possible types of motion that a controlled entity can have.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlledMotion<F: Float> {
    Stationary,
    ConstantVelocity(Vector3<F>),
}
