//! Controllers for user interaction

mod motion;

pub use motion::{
    MotionDirection, MotionState, NoMotionController, SemiDirectionalMotionController,
};

use nalgebra::{Rotation3, Translation3};

pub trait MotionController<F> {
    fn next_translation(&mut self) -> Option<Translation3<F>>;

    fn update_motion(&mut self, direction: MotionDirection, state: MotionState);

    fn set_orientation(&mut self, orientation: Rotation3<F>);

    fn rotate_orientation(&mut self, rotation: &Rotation3<F>);

    fn set_movement_speed(&mut self, movement_speed: F);

    fn stop(&mut self);
}
