//! Container for all data in the world.

use crate::{
    control::{MotionController, MotionDirection, MotionState},
    geometry::GeometricalData,
    rendering::RenderingSystem,
};

/// Container for all data required for simulating and
/// rendering the world.
pub struct World {
    geometrical_data: GeometricalData,
    renderer: RenderingSystem,
    motion_controller: Box<dyn MotionController<f32>>,
}

impl World {
    /// Creates a new world data container.
    pub fn new(
        geometrical_data: GeometricalData,
        renderer: RenderingSystem,
        controller: impl 'static + MotionController<f32>,
    ) -> Self {
        Self {
            geometrical_data,
            renderer,
            motion_controller: Box::new(controller),
        }
    }

    /// Returns the renderer.
    pub fn renderer(&self) -> &RenderingSystem {
        &self.renderer
    }

    /// Returns the renderer for mutation.
    pub fn renderer_mut(&mut self) -> &mut RenderingSystem {
        &mut self.renderer
    }

    /// Updates the motion controller with the given motion
    /// and propagates the information to the rest of the
    /// system.
    pub fn update_motion_controller(&mut self, state: MotionState, direction: MotionDirection) {
        self.motion_controller.update_motion(state, direction);
        if let Some(translation) = self.motion_controller.next_translation() {
            self.geometrical_data.transform_cameras(&translation.into());
            self.update();
        }
    }

    fn update(&mut self) {
        self.renderer.sync_with_geometry(&mut self.geometrical_data);
    }
}
