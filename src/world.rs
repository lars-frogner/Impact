//! Container for all data in the world.

use crate::{
    control::{MotionController, MotionDirection, MotionState},
    geometry::GeometricalData,
    rendering::RenderingSystem,
};
use std::sync::{Mutex, RwLock};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    geometrical_data: RwLock<GeometricalData>,
    renderer: RwLock<RenderingSystem>,
    motion_controller: Mutex<Box<dyn MotionController<f32>>>,
}

impl World {
    /// Creates a new world data container.
    pub fn new(
        geometrical_data: GeometricalData,
        renderer: RenderingSystem,
        controller: impl 'static + MotionController<f32>,
    ) -> Self {
        Self {
            geometrical_data: RwLock::new(geometrical_data),
            renderer: RwLock::new(renderer),
            motion_controller: Mutex::new(Box::new(controller)),
        }
    }

    pub fn geometrical_data(&self) -> &RwLock<GeometricalData> {
        &self.geometrical_data
    }

    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Updates the motion controller with the given motion.
    pub fn update_motion_controller(&self, state: MotionState, direction: MotionDirection) {
        let mut motion_controller = self.motion_controller.lock().unwrap();

        motion_controller.update_motion(state, direction);

        if let Some(translation) = motion_controller.next_translation() {
            drop(motion_controller); // Don't hold lock longer than neccessary

            self.geometrical_data
                .write()
                .unwrap()
                .transform_cameras(&translation.into());
        }
    }
}
